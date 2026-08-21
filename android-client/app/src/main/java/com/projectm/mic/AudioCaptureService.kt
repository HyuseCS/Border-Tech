package com.projectm.mic

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
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
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
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
import java.util.concurrent.atomic.AtomicReference
import java.util.Date
import javax.net.ssl.KeyManagerFactory
import javax.net.ssl.SSLContext
import javax.net.ssl.SSLServerSocket
import javax.net.ssl.SSLServerSocketFactory
import javax.net.ssl.SSLSocket

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
        const val ACTION_STOP = "com.projectm.mic.ACTION_STOP"
        private const val HANDSHAKE_TIMEOUT_MS = 10_000
        private const val WATCHDOG_TIMEOUT_MS = 5_000L
        private const val WATCHDOG_POLL_MS = 1_000L
        private const val ACCEPT_RETRY_DELAY_MS = 200L
        private const val MAX_ACCEPT_FAILURES = 5
        
        var state = mutableStateOf(ConnectionState.DISCONNECTED)
        var errorMessage = mutableStateOf("")
        var isServiceRunning = mutableStateOf(false)
        var authPin = mutableStateOf(String.format("%06d", SecureRandom().nextInt(1000000)))
        
        private var failedAttempts = 0
        private var lockoutUntil = 0L

        private fun handleAuthFailure() {
            failedAttempts++
            if (failedAttempts >= 6) {
                lockoutUntil = System.currentTimeMillis() + 5 * 60 * 1000L
                Log.w(TAG, "5 minute brute-force lockout engaged")
            } else if (failedAttempts >= 3) {
                lockoutUntil = System.currentTimeMillis() + 30 * 1000L
                Log.w(TAG, "30 second brute-force lockout engaged")
            }
        }

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
    private val audioRecord = AtomicReference<AudioRecord?>(null)
    @Volatile private var lastSuccessfulWriteMs: Long = System.currentTimeMillis()
    // Written on the client thread (handleClient catch), read on Main (accept-loop notification
    // update), so @Volatile is required. Lets the accept loop tell "listening for a first
    // connection" apart from "the link just dropped" without holding `state` at a misleading ERROR.
    @Volatile private var lastClientFailed: Boolean = false

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
        // Must be the FIRST statement: the rest of this function unconditionally re-notifies,
        // sets CONNECTING and relaunches the server loop, which would restart the very
        // connection this intent was sent to stop.
        if (intent?.action == ACTION_STOP) {
            Log.d(TAG, "onStartCommand: ACTION_STOP received")
            stopService(this)
            return START_NOT_STICKY
        }

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
        // Tapping the notification body stops the connection. FLAG_IMMUTABLE is required at targetSdk 34.
        val stopIntent = Intent(this, AudioCaptureService::class.java).setAction(ACTION_STOP)
        val stopPendingIntent = PendingIntent.getService(
            this,
            0,
            stopIntent,
            PendingIntent.FLAG_IMMUTABLE
        )
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("Sonus Audio Server")
            .setContentText("$text \u2022 Tap to stop")
            .setContentIntent(stopPendingIntent)
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
            val bindAddress = if (isUsb) java.net.InetAddress.getByName("127.0.0.1") else null
            serverSocket = ssf.createServerSocket(port, 50, bindAddress) as SSLServerSocket
            (serverSocket as SSLServerSocket).apply {
                needClientAuth = false
                enabledProtocols = arrayOf("TLSv1.3", "TLSv1.2")
            }

            Log.d(TAG, "Server socket ready, entering accept loop")
            var consecutiveAcceptFailures = 0
            while (isServiceRunning.value) {
                serviceScope.launch(Dispatchers.Main) {
                    state.value = ConnectionState.CONNECTING
                    val modeText = if (isUsb) "USB" else "Wi-Fi"
                    updateNotification(
                        if (lastClientFailed) "Connection lost - waiting for $modeText reconnect on port $port"
                        else "Waiting for $modeText connection on port $port"
                    )
                }

                val clientSocket = try {
                    serverSocket?.accept()
                } catch (e: Exception) {
                    Log.e(TAG, "Accept error: ${e.message}")
                    null
                }

                if (clientSocket == null) {
                    // One bad peer must not kill the server. Retry, but bail out after a bounded
                    // run of consecutive failures so a permanently dead socket cannot hot-spin.
                    consecutiveAcceptFailures++
                    if (consecutiveAcceptFailures >= MAX_ACCEPT_FAILURES) {
                        Log.e(TAG, "Giving up after $consecutiveAcceptFailures consecutive accept failures")
                        break
                    }
                    Thread.sleep(ACCEPT_RETRY_DELAY_MS)
                    continue
                }
                consecutiveAcceptFailures = 0

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
        var watchdogJob: Job? = null
        try {
            // Guards every blocking read of the pre-streaming phase: the TLS handshake below and
            // all SRP reads. soTimeout is a persistent socket property, so setting it once here is
            // enough; a SocketTimeoutException propagates to the general catch and the finally block
            // closes the socket, exactly like any other handshake failure.
            socket.soTimeout = HANDSHAKE_TIMEOUT_MS

            if (System.currentTimeMillis() < lockoutUntil) {
                Log.w(TAG, "Connection rejected: Active lockout")
                socket.close()
                return
            }
            
            if (socket is SSLSocket) {
                Log.d(TAG, "Starting TLS handshake with ${socket.inetAddress}...")
                socket.startHandshake()
                Log.d(TAG, "TLS handshake successful")
            }

            Log.d(TAG, "Client connected: ${socket.inetAddress}")
            socket.tcpNoDelay = true
            val outputStream = socket.getOutputStream()
            val inputStream = socket.getInputStream()
            
            // SRP Authentication
            val authHeader = ByteArray(4)
            var bytesRead = inputStream.read(authHeader)
            if (bytesRead != 4 || String(authHeader) != "SRP1") {
                Log.e(TAG, "Invalid SRP1 header: ${if (bytesRead > 0) String(authHeader.sliceArray(0 until bytesRead)) else "EMPTY"}")
                handleAuthFailure()
                socket.close()
                return
            }
            
            val aBytes = ByteArray(256)
            var totalRead = 0
            while (totalRead < 256) {
                val r = inputStream.read(aBytes, totalRead, 256 - totalRead)
                if (r == -1) break
                totalRead += r
            }
            if (totalRead != 256) {
                Log.e(TAG, "Failed to read SRP A bytes")
                handleAuthFailure()
                socket.close()
                return
            }
            
            val A = java.math.BigInteger(1, aBytes)
            
            val digest = org.bouncycastle.crypto.digests.SHA256Digest()
            val group = org.bouncycastle.crypto.agreement.srp.SRP6StandardGroups.rfc5054_2048
            val vGen = org.bouncycastle.crypto.agreement.srp.SRP6VerifierGenerator()
            vGen.init(group.n, group.g, digest)
            val salt = ByteArray(16)
            SecureRandom().nextBytes(salt)
            val identity = "client".toByteArray()
            val password = authPin.value.toByteArray()
            val verifier = vGen.generateVerifier(salt, identity, password)
            
            val srpServer = org.bouncycastle.crypto.agreement.srp.SRP6Server()
            srpServer.init(group.n, group.g, verifier, digest, SecureRandom())
            val B = srpServer.generateServerCredentials()
            val bBytesArray = B.toByteArray()
            val bPad = ByteArray(256)
            val bOffset = Math.max(0, bBytesArray.size - 256)
            val bLen = Math.min(256, bBytesArray.size)
            System.arraycopy(bBytesArray, bOffset, bPad, 256 - bLen, bLen)
            
            outputStream.write("SRP2".toByteArray())
            outputStream.write(salt)
            outputStream.write(bPad)
            outputStream.flush()
            
            val m1Header = ByteArray(4)
            if (inputStream.read(m1Header) != 4 || String(m1Header) != "SRP3") {
                Log.e(TAG, "Invalid SRP3 header")
                handleAuthFailure()
                socket.close()
                return
            }
            
            val m1Bytes = ByteArray(32)
            totalRead = 0
            while (totalRead < 32) {
                val r = inputStream.read(m1Bytes, totalRead, 32 - totalRead)
                if (r == -1) break
                totalRead += r
            }
            
            val secret = srpServer.calculateSecret(A)
            
            val sBytesArray = secret.toByteArray()
            val sPad = ByteArray(256)
            val sOffset = Math.max(0, sBytesArray.size - 256)
            val sLen = Math.min(256, sBytesArray.size)
            System.arraycopy(sBytesArray, sOffset, sPad, 256 - sLen, sLen)
            
            val md = MessageDigest.getInstance("SHA-256")
            md.update("M1".toByteArray())
            val expectedM1 = md.digest(sPad)

            if (!MessageDigest.isEqual(m1Bytes, expectedM1)) {
                Log.e(TAG, "Invalid Custom SRP M1 attempt from ${socket.inetAddress}")
                handleAuthFailure()
                socket.close()
                return
            }
            
            // Authentication Success
            failedAttempts = 0
            lockoutUntil = 0L
            
            md.reset()
            md.update("M2".toByteArray())
            val m2Pad = md.digest(sPad)
            
            outputStream.write("SRP4".toByteArray())
            outputStream.write(m2Pad)
            outputStream.flush()
            
            // Streaming only writes, and soTimeout affects reads only, so a lingering handshake
            // timeout would be inert here. Cleared anyway as defensive hygiene.
            socket.soTimeout = 0

            val peerAddr = socket.inetAddress.hostAddress
            lastClientFailed = false
            serviceScope.launch(Dispatchers.Main) {
                state.value = ConnectionState.CONNECTED
                updateNotification("Streaming audio to $peerAddr")
            }

            // Audio Record configuration
            val sampleRate = 48000
            val channelConfig = AudioFormat.CHANNEL_IN_MONO
            val audioFormat = AudioFormat.ENCODING_PCM_16BIT
            
            val minBufferSize = AudioRecord.getMinBufferSize(sampleRate, channelConfig, audioFormat)
            val bufferSize = minBufferSize.coerceAtLeast(480 * 2)

            @Suppress("MissingPermission")
            val recorder = AudioRecord(
                MediaRecorder.AudioSource.MIC,
                sampleRate,
                channelConfig,
                audioFormat,
                bufferSize
            )
            audioRecord.set(recorder)

            if (recorder.state != AudioRecord.STATE_INITIALIZED) {
                throw IllegalStateException("Failed to initialize AudioRecord")
            }

            recorder.startRecording()
            Log.d(TAG, "AudioRecord started, streaming...")
            
            // Two independent thresholds, deliberately different:
            //  - 50ms (in the loop below): codec-quality heuristic. Three consecutive slow writes
            //    degrade the stream to 24kHz to shed bandwidth. NOT a liveness signal.
            //  - 5s (WATCHDOG_TIMEOUT_MS): liveness signal. Ordinary Wi-Fi jitter is already absorbed
            //    by the 24kHz degradation valve, so a 5s gap with no completed write means the socket
            //    is dead, not merely congested. 5s leaves headroom against scheduling jitter while
            //    still recovering promptly.
            // Reset before launching: this field is service-level and shared across sequential client
            // connections, so a stale timestamp from a previous connection would force-close this
            // brand-new socket before it ever writes its first frame.
            lastSuccessfulWriteMs = System.currentTimeMillis()
            // Closing the socket from ANOTHER thread is the only thing that unblocks a thread parked
            // inside a blocked write(); captureJob.cancel() only trips cooperative coroutine
            // cancellation and would not reach this loop at all.
            watchdogJob = serviceScope.launch(Dispatchers.IO) {
                while (isActive) {
                    delay(WATCHDOG_POLL_MS)
                    if (System.currentTimeMillis() - lastSuccessfulWriteMs > WATCHDOG_TIMEOUT_MS) {
                        Log.w(TAG, "Watchdog: no successful write for ${WATCHDOG_TIMEOUT_MS}ms, closing socket")
                        try { socket.close() } catch (e: Exception) {}
                        break
                    }
                }
            }

            val buffer = ByteArray(480)
            var seqNum: Short = 0
            var writeStallsCount = 0
            var writeOkCount = 0
            var isDegraded = false

            // Streaming loop for this client.
            // NOTE: this loop deliberately uses the LOCAL `recorder` reference, not audioRecord.get().
            // When another thread tears down via audioRecord.getAndSet(null)?.let { stop(); release() },
            // the next recorder.read() on the released instance throws IllegalStateException, which the
            // catch below turns into a clean, immediate loop exit. Routing through audioRecord.get()
            // would yield null and silently skip reads (a spin) instead of failing fast. Do not "fix" this.
            while (isServiceRunning.value && socket.isConnected && !socket.isClosed) {
                val bytesRead = recorder.read(buffer, 0, buffer.size)
                if (bytesRead > 0) {
                    val payloadToSend: ByteArray
                    val lengthToSend: Int
                    if (isDegraded) {
                        payloadToSend = downsampleTo24kHz(buffer, bytesRead)
                        lengthToSend = payloadToSend.size
                    } else {
                        payloadToSend = buffer
                        lengthToSend = bytesRead
                    }

                    val startTime = System.currentTimeMillis()
                    try {
                        sendAudioPacket(outputStream, payloadToSend, lengthToSend, seqNum, isDegraded)
                        outputStream.flush()
                    } catch (e: Exception) {
                        Log.e(TAG, "Socket write error", e)
                        break
                    }
                    lastSuccessfulWriteMs = System.currentTimeMillis()
                    val duration = System.currentTimeMillis() - startTime
                    
                    seqNum = (seqNum + 1).toShort()

                    if (duration > 50) {
                        writeStallsCount++
                        writeOkCount = 0
                        if (writeStallsCount >= 3 && !isDegraded) {
                            isDegraded = true
                            Log.w(TAG, "TCP write stall detected (${duration}ms for ${writeStallsCount} frames). Degrading to 24kHz.")
                        }
                    } else {
                        writeStallsCount = 0
                        if (duration <= 10) {
                            writeOkCount++
                            if (writeOkCount >= 10 && isDegraded) {
                                isDegraded = false
                                Log.i(TAG, "TCP backpressure cleared (10 frames sent in <= 10ms). Restoring to 48kHz.")
                            }
                        } else {
                            writeOkCount = 0
                        }
                    }
                } else if (bytesRead < 0) {
                    Log.w(TAG, "AudioRecord read error: $bytesRead")
                    break
                }
            }

        } catch (e: Exception) {
            Log.e(TAG, "Handle client error", e)
            lastClientFailed = true
            // Queued on Main before the finally block's own Main-queued update, so the guard
            // there (state != ERROR) sees this value and does not overwrite it with CONNECTING.
            serviceScope.launch(Dispatchers.Main) {
                state.value = ConnectionState.ERROR
                errorMessage.value = e.localizedMessage ?: e.javaClass.simpleName
            }
        } finally {
            Log.d(TAG, "Cleaning up client connection")
            // Sibling of captureJob, not a child: captureJob.cancel() does not reach it.
            watchdogJob?.cancel()
            try { socket.close() } catch (e: Exception) {}
            audioRecord.getAndSet(null)?.let {
                try { it.stop() } catch (e: Exception) {}
                try { it.release() } catch (e: Exception) {}
            }
            
            serviceScope.launch(Dispatchers.Main) {
                if (isServiceRunning.value && state.value != ConnectionState.ERROR) {
                    state.value = ConnectionState.CONNECTING
                }
            }
        }
    }

    private fun sendAudioPacket(out: OutputStream, pcmData: ByteArray, length: Int, seqNum: Short, is24khz: Boolean) {
        val verFlags = ((1 shl 4) or (if (is24khz) 1 else 0)).toByte()
        val header = byteArrayOf(
            'M'.code.toByte(),
            'C'.code.toByte(),
            verFlags,
            (seqNum.toInt() shr 8).toByte(),
            (seqNum.toInt() and 0xFF).toByte(),
            (length shr 8).toByte(),
            (length and 0xFF).toByte()
        )
        out.write(header)
        out.write(pcmData, 0, length)
    }

    private fun downsampleTo24kHz(pcm48: ByteArray, length48: Int): ByteArray {
        val numSamples48 = length48 / 2
        val numSamples24 = numSamples48 / 2
        val pcm24 = ByteArray(numSamples24 * 2)
        for (i in 0 until numSamples24) {
            // Copy 16-bit sample (2 bytes) from index i*4 to i*2
            pcm24[i * 2] = pcm48[i * 4]
            pcm24[i * 2 + 1] = pcm48[i * 4 + 1]
        }
        return pcm24
    }

    private fun setupSslContext(): SSLContext {
        val keystoreFile = java.io.File(filesDir, "keystore.p12")
        val passwordChars = "sonus_keystore_pass".toCharArray()
        val keyStore = KeyStore.getInstance("PKCS12")

        if (keystoreFile.exists()) {
            Log.i(TAG, "Loading persistent keystore from ${keystoreFile.absolutePath}")
            java.io.FileInputStream(keystoreFile).use { fis ->
                keyStore.load(fis, passwordChars)
            }
        } else {
            Log.i(TAG, "Generating new persistent keystore...")
            // Generate ephemeral key pair using ECDSA (secp256r1) for maximum compatibility
            val keyPairGenerator = KeyPairGenerator.getInstance("EC", "BC")
            keyPairGenerator.initialize(256, SecureRandom())
            val keyPair = keyPairGenerator.generateKeyPair()

            // Generate self-signed certificate
            val cert = generateSelfSignedCertificate(keyPair)

            // Save to keystore file
            keyStore.load(null, null)
            keyStore.setKeyEntry("sonus-key", keyPair.private, passwordChars, arrayOf(cert))
            java.io.FileOutputStream(keystoreFile).use { fos ->
                keyStore.store(fos, passwordChars)
            }
        }

        // Set up KeyManagerFactory
        val kmf = KeyManagerFactory.getInstance(KeyManagerFactory.getDefaultAlgorithm())
        kmf.init(keyStore, passwordChars)

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
        
        // Use SHA256withECDSA for the signature
        val signer = JcaContentSignerBuilder("SHA256withECDSA").setProvider("BC").build(keyPair.private)
        return JcaX509CertificateConverter().setProvider("BC").getCertificate(certBuilder.build(signer))
    }

    private fun cleanup() {
        Log.d(TAG, "Service cleanup")
        audioRecord.getAndSet(null)?.let {
            try {
                it.stop()
            } catch (_: Exception) {}
            try {
                it.release()
            } catch (_: Exception) {}
        }

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
