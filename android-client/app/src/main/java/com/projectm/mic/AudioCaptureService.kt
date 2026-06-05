package com.projectm.mic

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import android.os.Build
import android.os.IBinder
import androidx.compose.runtime.mutableStateOf
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import java.io.OutputStream
import java.net.InetSocketAddress
import java.net.Socket

class AudioCaptureService : Service() {

    enum class ConnectionState {
        DISCONNECTED,
        CONNECTING,
        CONNECTED,
        ERROR
    }

    companion object {
        const val NOTIFICATION_ID = 1001
        const val CHANNEL_ID = "audio_capture_channel"
        
        var state = mutableStateOf(ConnectionState.DISCONNECTED)
        var errorMessage = mutableStateOf("")
        var isServiceRunning = mutableStateOf(false)

        fun startService(context: Context, ip: String, port: Int) {
            val intent = Intent(context, AudioCaptureService::class.java).apply {
                putExtra("EXTRA_IP", ip)
                putExtra("EXTRA_PORT", port)
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }

        fun stopService(context: Context) {
            val intent = Intent(context, AudioCaptureService::class.java)
            context.stopService(intent)
        }
    }

    private val serviceScope = CoroutineScope(Dispatchers.Main + Job())
    private var captureJob: Job? = null
    private var socket: Socket? = null
    private var audioRecord: AudioRecord? = null

    override fun onCreate() {
        super.onCreate()
        isServiceRunning.value = true
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val ip = intent?.getStringExtra("EXTRA_IP") ?: "127.0.0.1"
        val port = intent?.getIntExtra("EXTRA_PORT", 47999) ?: 47999

        startForegroundNotification()

        state.value = ConnectionState.CONNECTING
        errorMessage.value = ""

        captureJob?.cancel()
        captureJob = serviceScope.launch(Dispatchers.IO) {
            runCaptureLoop(ip, port)
        }

        return START_NOT_STICKY
    }

    private fun startForegroundNotification() {
        val notification = createNotification("Connecting to PC client...")
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            startForeground(
                NOTIFICATION_ID, 
                notification, 
                ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE
            )
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
    }

    private fun updateNotification(text: String) {
        val notificationManager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        notificationManager.notify(NOTIFICATION_ID, createNotification(text))
    }

    private fun createNotification(contentText: String): Notification {
        val builder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Notification.Builder(this, CHANNEL_ID)
        } else {
            @Suppress("DEPRECATION")
            Notification.Builder(this)
        }

        return builder
            .setContentTitle("Project-M Microphone")
            .setContentText(contentText)
            .setSmallIcon(R.drawable.ic_launcher_mic)
            .setOngoing(true)
            .build()
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Microphone Streaming Status",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Shows status of the active mic stream to PC"
            }
            val manager = getSystemService(NotificationManager::class.java)
            manager.createNotificationChannel(channel)
        }
    }

    private fun runCaptureLoop(ip: String, port: Int) {
        try {
            socket = Socket().apply {
                tcpNoDelay = true
                sendBufferSize = 64 * 1024
                connect(InetSocketAddress(ip, port), 5000)
            }
            
            val outputStream = socket!!.getOutputStream()
            
            serviceScope.launch(Dispatchers.Main) {
                state.value = ConnectionState.CONNECTED
                updateNotification("Streaming microphone audio to $ip:$port")
            }

            // Audio Record configuration
            val sampleRate = 48000
            val channelConfig = AudioFormat.CHANNEL_IN_MONO
            val audioFormat = AudioFormat.ENCODING_PCM_16BIT
            
            val minBufferSize = AudioRecord.getMinBufferSize(sampleRate, channelConfig, audioFormat)
            val bufferSize = minBufferSize.coerceAtLeast(960 * 2)

            @Suppress("MissingPermission")
            audioRecord = AudioRecord(
                MediaRecorder.AudioSource.MIC,
                sampleRate,
                channelConfig,
                audioFormat,
                bufferSize
            )

            if (audioRecord!!.state != AudioRecord.STATE_INITIALIZED) {
                throw IllegalStateException("Failed to initialize AudioRecord")
            }

            audioRecord!!.startRecording()
            
            // 960 bytes = 480 samples = 10ms of 16-bit Mono @ 48kHz
            val buffer = ByteArray(960)

            while (socket != null && socket!!.isConnected && audioRecord != null) {
                val bytesRead = audioRecord!!.read(buffer, 0, buffer.size)
                if (bytesRead > 0) {
                    sendAudioPacket(outputStream, buffer, bytesRead)
                } else if (bytesRead < 0) {
                    throw IllegalStateException("AudioRecord read error: $bytesRead")
                }
            }

        } catch (e: Exception) {
            e.printStackTrace()
            serviceScope.launch(Dispatchers.Main) {
                state.value = ConnectionState.ERROR
                errorMessage.value = e.localizedMessage ?: "Unknown connection error"
                updateNotification("Connection Error: ${errorMessage.value}")
            }
        } finally {
            cleanup()
        }
    }

    private fun sendAudioPacket(out: OutputStream, pcmData: ByteArray, length: Int) {
        // Frame format: 'M' (1B), 'C' (1B), payload length high (1B), payload length low (1B) + payload
        val header = byteArrayOf(
            'M'.code.toByte(),
            'C'.code.toByte(),
            ((length shr 8) and 0xFF).toByte(),
            (length and 0xFF).toByte()
        )
        out.write(header)
        out.write(pcmData, 0, length)
    }

    private fun cleanup() {
        try {
            audioRecord?.stop()
        } catch (_: Exception) {}
        try {
            audioRecord?.release()
        } catch (_: Exception) {}
        audioRecord = null

        try {
            socket?.close()
        } catch (_: Exception) {}
        socket = null

        serviceScope.launch(Dispatchers.Main) {
            if (state.value != ConnectionState.ERROR) {
                state.value = ConnectionState.DISCONNECTED
            }
            isServiceRunning.value = false
        }
    }

    override fun onDestroy() {
        captureJob?.cancel()
        cleanup()
        isServiceRunning.value = false
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null
}
