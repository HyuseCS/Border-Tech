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
import android.util.Log
import androidx.compose.runtime.mutableStateOf
import androidx.core.app.NotificationCompat
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import org.bouncycastle.asn1.x500.X500Name
import org.bouncycastle.cert.jcajce.JcaX509CertificateConverter
import org.bouncycastle.cert.jcajce.JcaX509v3CertificateBuilder
import org.bouncycastle.jce.provider.BouncyCastleProvider
import org.bouncycastle.operator.jcajce.JcaContentSignerBuilder
import java.io.OutputStream
import java.math.BigInteger
import java.net.ServerSocket
import java.net.Socket
import java.security.*
import java.security.cert.X509Certificate
import java.util.Date
import javax.net.ssl.KeyManagerFactory
import javax.net.ssl.SSLContext
import javax.net.ssl.SSLServerSocket
import javax.net.ssl.SSLServerSocketFactory

class AudioCaptureService : Service() {

    enum class ConnectionState {
        DISCONNECTED,
        CONNECTING,
        CONNECTED,
        ERROR
    }

    companion object {
        private const val TAG = "AudioCaptureService"
        const val NOTIFICATION_ID = 1001
        const val CHANNEL_ID = "audio_capture_channel"
        
        var state = mutableStateOf(ConnectionState.DISCONNECTED)
        var errorMessage = mutableStateOf("")
        var isServiceRunning = mutableStateOf(false)

        fun startService(context: Context, port: Int, isUsb: Boolean) {
            val intent = Intent(context, AudioCaptureService::class.java).apply {
                putExtra("EXTRA_PORT", port)
                putExtra("EXTRA_IS_USB", isUsb)
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
    private var serverSocket: ServerSocket? = null
    private var audioRecord: AudioRecord? = null

    override fun onCreate() {
        super.onCreate()
        Log.d(TAG, "Service onCreate")
        isServiceRunning.value = true
        createNotificationChannel()
        
        // Register BouncyCastle Provider
        Security.removeProvider("BC")
        Security.addProvider(BouncyCastleProvider())
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val port = intent?.getIntExtra("EXTRA_PORT", 47999) ?: 47999
        val isUsb = intent?.getBooleanExtra("EXTRA_IS_USB", true) ?: true

        Log.d(TAG, "onStartCommand: port=$port, isUsb=$isUsb")
        startForegroundNotification()

        state.value = ConnectionState.CONNECTING
        errorMessage.value = ""

        captureJob?.cancel()
        captureJob = serviceScope.launch(Dispatchers.IO) {
            runServerLoop(port, isUsb)
        }

        return START_NOT_STICKY
    }

    private fun startForegroundNotification() {
        val notification = createNotification("Initializing Sonus...")
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

    private fun createNotification(text: String): Notification {
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("Sonus Audio Server")
            .setContentText(text)
            .setSmallIcon(R.drawable.ic_launcher_mic)
            .setOngoing(true)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setForegroundServiceBehavior(NotificationCompat.FOREGROUND_SERVICE_IMMEDIATE)
            .build()
    }

    private fun updateNotification(text: String) {
        val notification = createNotification(text)
        val manager = getSystemService(NotificationManager::class.java)
        manager.notify(NOTIFICATION_ID, notification)
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Microphone Streaming Status",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Shows status of the active mic server"
            }
            val manager = getSystemService(NotificationManager::class.java)
            manager.createNotificationChannel(channel)
        }
    }

    private fun runServerLoop(port: Int, isUsb: Boolean) {
        try {
            Log.d(TAG, "Setting up SSL Context...")
            val sslContext = setupSslContext()
            val ssf: SSLServerSocketFactory = sslContext.serverSocketFactory
            
            Log.d(TAG, "Creating SSL Server Socket on port $port...")
            serverSocket = ssf.createServerSocket(port) as SSLServerSocket
            (serverSocket as SSLServerSocket).apply {
                needClientAuth = false
                enabledProtocols = arrayOf("TLSv1.3", "TLSv1.2")
            }

            Log.d(TAG, "Server socket ready, entering accept loop")
            while (isServiceRunning.value) {
                serviceScope.launch(Dispatchers.Main) {
                    state.value = ConnectionState.CONNECTING
                    val modeText = if (isUsb) "USB" else "Wi-Fi"
                    updateNotification("Waiting for $modeText connection on port $port")
                }

                val clientSocket = try {
                    serverSocket?.accept()
                } catch (e: Exception) {
                    Log.e(TAG, "Accept error: ${e.message}")
                    null
                } ?: break
                
                handleClient(clientSocket)
            }

        } catch (e: Exception) {
            Log.e(TAG, "Server loop error", e)
            serviceScope.launch(Dispatchers.Main) {
                state.value = ConnectionState.ERROR
                errorMessage.value = e.localizedMessage ?: e.javaClass.simpleName
                updateNotification("Server Error: ${errorMessage.value}")
            }
        } finally {
            cleanup()
        }
    }

    private fun handleClient(socket: Socket) {
        try {
            Log.d(TAG, "Client connected: ${socket.inetAddress}")
            socket.tcpNoDelay = true
            val outputStream = socket.getOutputStream()
            
            val peerAddr = socket.inetAddress.hostAddress
            serviceScope.launch(Dispatchers.Main) {
                state.value = ConnectionState.CONNECTED
                updateNotification("Streaming audio to $peerAddr")
            }

            // Audio Record configuration
            val sampleRate = 48000
            val channelConfig = AudioFormat.CHANNEL_IN_MONO
            val audioFormat = AudioFormat.ENCODING_PCM_16BIT
            
            val minBufferSize = AudioRecord.getMinBufferSize(sampleRate, channelConfig, audioFormat)
            val bufferSize = minBufferSize.coerceAtLeast(960 * 2)

            @Suppress("MissingPermission")
            val recorder = AudioRecord(
                MediaRecorder.AudioSource.MIC,
                sampleRate,
                channelConfig,
                audioFormat,
                bufferSize
            )
            audioRecord = recorder

            if (recorder.state != AudioRecord.STATE_INITIALIZED) {
                throw IllegalStateException("Failed to initialize AudioRecord")
            }

            recorder.startRecording()
            Log.d(TAG, "AudioRecord started, streaming...")
            
            val buffer = ByteArray(960)

            // Streaming loop for this client
            while (isServiceRunning.value && socket.isConnected && !socket.isClosed) {
                val bytesRead = recorder.read(buffer, 0, buffer.size)
                if (bytesRead > 0) {
                    sendAudioPacket(outputStream, buffer, bytesRead)
                } else if (bytesRead < 0) {
                    Log.w(TAG, "AudioRecord read error: $bytesRead")
                    break
                }
            }

        } catch (e: Exception) {
            Log.e(TAG, "Handle client error", e)
        } finally {
            Log.d(TAG, "Cleaning up client connection")
            try { socket.close() } catch (e: Exception) {}
            try { audioRecord?.stop() } catch (e: Exception) {}
            try { audioRecord?.release() } catch (e: Exception) {}
            audioRecord = null
            
            serviceScope.launch(Dispatchers.Main) {
                if (isServiceRunning.value) {
                    state.value = ConnectionState.CONNECTING
                }
            }
        }
    }

    private fun sendAudioPacket(out: OutputStream, pcmData: ByteArray, length: Int) {
        val header = byteArrayOf(
            'M'.code.toByte(),
            'C'.code.toByte(),
            (length shr 8).toByte(),
            (length and 0xFF).toByte()
        )
        out.write(header)
        out.write(pcmData, 0, length)
    }

    private fun setupSslContext(): SSLContext {
        // Generate ephemeral key pair
        val keyPairGenerator = KeyPairGenerator.getInstance("RSA", "BC")
        keyPairGenerator.initialize(2048)
        val keyPair = keyPairGenerator.generateKeyPair()

        // Generate self-signed certificate
        val cert = generateSelfSignedCertificate(keyPair)

        // Create a KeyStore (PKCS12 is better for in-memory)
        val keyStore = KeyStore.getInstance("PKCS12", "BC")
        keyStore.load(null, null)
        keyStore.setKeyEntry("sonus-key", keyPair.private, "password".toCharArray(), arrayOf(cert))

        // Set up KeyManagerFactory
        val kmf = KeyManagerFactory.getInstance(KeyManagerFactory.getDefaultAlgorithm())
        kmf.init(keyStore, "password".toCharArray())

        val sslContext = SSLContext.getInstance("TLS")
        sslContext.init(kmf.keyManagers, null, SecureRandom())
        return sslContext
    }

    private fun generateSelfSignedCertificate(keyPair: KeyPair): X509Certificate {
        val issuer = X500Name("CN=Sonus, O=ProjectM, L=Local, C=US")
        val serial = BigInteger.valueOf(System.currentTimeMillis())
        val notBefore = Date(System.currentTimeMillis() - 1000L * 60 * 60 * 24)
        val notAfter = Date(System.currentTimeMillis() + 1000L * 60 * 60 * 24 * 365)
        
        val certBuilder = JcaX509v3CertificateBuilder(
            issuer,
            serial,
            notBefore,
            notAfter,
            issuer,
            keyPair.public
        )
        
        val signer = JcaContentSignerBuilder("SHA256withRSA").setProvider("BC").build(keyPair.private)
        return JcaX509CertificateConverter().setProvider("BC").getCertificate(certBuilder.build(signer))
    }

    private fun cleanup() {
        Log.d(TAG, "Service cleanup")
        try {
            audioRecord?.stop()
        } catch (_: Exception) {}
        try {
            audioRecord?.release()
        } catch (_: Exception) {}
        audioRecord = null

        try {
            serverSocket?.close()
        } catch (_: Exception) {}
        serverSocket = null

        serviceScope.launch(Dispatchers.Main) {
            if (state.value != ConnectionState.ERROR) {
                state.value = ConnectionState.DISCONNECTED
            }
            isServiceRunning.value = false
        }
    }

    override fun onDestroy() {
        Log.d(TAG, "Service onDestroy")
        isServiceRunning.value = false
        captureJob?.cancel()
        cleanup()
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null
}
