package com.projectm.mic

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.LinearOutSlowInEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shadow
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat

class MainActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        setContent {
            MaterialTheme(
                colorScheme = MaterialTheme.colorScheme.copy(
                    background = Color(0xFF0F0F12),
                    surface = Color(0xFF1E1E24)
                )
            ) {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = Color(0xFF0F0F12)
                ) {
                    MainScreen(this)
                }
            }
        }
    }
}

@Composable
fun MainScreen(activity: ComponentActivity) {
    val sharedPref = activity.getSharedPreferences("sonus_pref", Context.MODE_PRIVATE)

    var portString by remember { mutableStateOf(sharedPref.getString("port", "47999") ?: "47999") }
    var localIp by remember { mutableStateOf("Fetching IP...") }

    val connectionState by AudioCaptureService.state
    val isRunning by AudioCaptureService.isServiceRunning
    val errorMessage by AudioCaptureService.errorMessage

    var hasMicPermission by remember {
        mutableStateOf(
            ContextCompat.checkSelfPermission(activity, Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED
        )
    }

    val launcher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.RequestMultiplePermissions()
    ) { permissions ->
        hasMicPermission = permissions[Manifest.permission.RECORD_AUDIO] ?: false
        if (!hasMicPermission) {
            Toast.makeText(activity, "Microphone permission is required to stream audio", Toast.LENGTH_LONG).show()
        }
    }

    LaunchedEffect(Unit) {
        val permissions = mutableListOf(Manifest.permission.RECORD_AUDIO)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            permissions.add(Manifest.permission.POST_NOTIFICATIONS)
        }
        launcher.launch(permissions.toTypedArray())
    }

    // Refresh IP address
    LaunchedEffect(isRunning) {
        localIp = getLocalIpAddress(activity)
    }

    // Save preferences when Port updates
    LaunchedEffect(portString) {
        sharedPref.edit().apply {
            putString("port", portString)
            apply()
        }
    }

    val neonColor = when (connectionState) {
        AudioCaptureService.ConnectionState.CONNECTED -> Color(0xFF00FFCC)
        AudioCaptureService.ConnectionState.CONNECTING -> Color(0xFFFFCC00)
        AudioCaptureService.ConnectionState.ERROR -> Color(0xFFFF3366)
        AudioCaptureService.ConnectionState.DISCONNECTED -> Color(0xFF888899)
    }

    val btnColor by animateColorAsState(
        targetValue = if (isRunning) {
            if (connectionState == AudioCaptureService.ConnectionState.ERROR) Color(0xFFFF3366) 
            else if (connectionState == AudioCaptureService.ConnectionState.CONNECTED) Color(0xFF00FFCC)
            else Color(0xFFFFCC00)
        } else Color(0xFF333344),
        animationSpec = tween(500),
        label = "btnColor"
    )

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.SpaceBetween
    ) {
        // App Header
        Column(
            modifier = Modifier.fillMaxWidth().padding(top = 16.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Image(
                painter = painterResource(id = R.drawable.ic_launcher_mic),
                contentDescription = "Sonus Logo",
                modifier = Modifier.size(80.dp).padding(bottom = 12.dp)
            )
            Text(
                text = "SONUS",
                fontSize = 32.sp,
                fontWeight = FontWeight.Black,
                fontFamily = FontFamily.Monospace,
                color = Color.White,
                style = MaterialTheme.typography.titleLarge.copy(
                    shadow = Shadow(
                        color = Color(0x6600FFCC),
                        blurRadius = 15f
                    )
                )
            )
            Text(
                text = "SECURE AUDIO SERVER",
                fontSize = 12.sp,
                color = Color(0xFF8E8E9F),
                fontFamily = FontFamily.SansSerif,
                letterSpacing = 2.sp
            )
        }

        // Connection Status Box
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(16.dp))
                .background(Color(0xFF1E1E24))
                .padding(20.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Text(
                text = when (connectionState) {
                    AudioCaptureService.ConnectionState.CONNECTED -> "SECURELY STREAMING"
                    AudioCaptureService.ConnectionState.CONNECTING -> "LISTENING FOR PC..."
                    AudioCaptureService.ConnectionState.ERROR -> "SYSTEM ERROR"
                    AudioCaptureService.ConnectionState.DISCONNECTED -> "SYSTEM READY"
                },
                color = neonColor,
                fontSize = 14.sp,
                fontWeight = FontWeight.Bold,
                letterSpacing = 2.sp
            )
            
            Spacer(modifier = Modifier.height(8.dp))
            
            Text(
                text = if (isRunning) localIp else "OFFLINE",
                color = Color.White,
                fontSize = 24.sp,
                fontWeight = FontWeight.Light
            )

            if (connectionState == AudioCaptureService.ConnectionState.ERROR && errorMessage.isNotEmpty()) {
                Text(
                    text = errorMessage,
                    color = Color(0xFFFF3366),
                    fontSize = 12.sp,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.padding(top = 8.dp)
                )
            }
        }

        // Main Power Button
        Box(
            contentAlignment = Alignment.Center,
            modifier = Modifier.size(200.dp)
        ) {
            // Pulse Effect
            if (isRunning && connectionState != AudioCaptureService.ConnectionState.ERROR) {
                val infiniteTransition = rememberInfiniteTransition(label = "pulse")
                val scale by infiniteTransition.animateFloat(
                    initialValue = 1f,
                    targetValue = 1.4f,
                    animationSpec = infiniteRepeatable(
                        animation = tween(1500, easing = LinearOutSlowInEasing),
                        repeatMode = RepeatMode.Restart
                    ),
                    label = "scale"
                )
                val alpha by infiniteTransition.animateFloat(
                    initialValue = 0.4f,
                    targetValue = 0f,
                    animationSpec = infiniteRepeatable(
                        animation = tween(1500, easing = LinearOutSlowInEasing),
                        repeatMode = RepeatMode.Restart
                    ),
                    label = "alpha"
                )
                
                Box(
                    modifier = Modifier
                        .size(140.dp)
                        .scale(scale)
                        .clip(CircleShape)
                        .background(neonColor.copy(alpha = alpha))
                )
            }

            Button(
                onClick = {
                    if (!hasMicPermission) {
                        Toast.makeText(activity, "Permission denied", Toast.LENGTH_SHORT).show()
                        return@Button
                    }
                    if (isRunning) {
                        AudioCaptureService.stopService(activity)
                    } else {
                        val port = portString.toIntOrNull() ?: 47999
                        AudioCaptureService.startService(activity, port)
                    }
                },
                modifier = Modifier
                    .size(140.dp)
                    .clip(CircleShape),
                colors = ButtonDefaults.buttonColors(
                    containerColor = Color(0xFF1E1E24)
                ),
                shape = CircleShape,
                border = androidx.compose.foundation.BorderStroke(4.dp, btnColor)
            ) {
                Text(
                    text = if (isRunning) "STOP" else "START",
                    fontSize = 20.sp,
                    fontWeight = FontWeight.ExtraBold,
                    color = Color.White,
                    letterSpacing = 1.5.sp
                )
            }
        }

        // Port Settings
        Column(modifier = Modifier.fillMaxWidth()) {
            Text(
                text = "SERVER SETTINGS",
                color = Color(0xFF8E8E9F),
                fontSize = 12.sp,
                fontWeight = FontWeight.Bold,
                letterSpacing = 1.sp,
                modifier = Modifier.padding(bottom = 8.dp)
            )
            
            OutlinedTextField(
                value = portString,
                onValueChange = { portString = it },
                label = { Text("Server Port") },
                singleLine = true,
                enabled = !isRunning,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = Color(0xFF00FFCC),
                    unfocusedBorderColor = Color(0xFF333344),
                    focusedLabelColor = Color(0xFF00FFCC),
                    unfocusedLabelColor = Color(0xFF8E8E9F),
                    focusedTextColor = Color.White,
                    unfocusedTextColor = Color.White
                ),
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(12.dp)
            )
        }

        // Footer info
        Text(
            text = "Stream Settings: 48kHz, 16-bit PCM, Mono",
            fontSize = 11.sp,
            color = Color(0xFF555566),
            modifier = Modifier.padding(bottom = 8.dp)
        )
    }
}

fun getLocalIpAddress(context: Context): String {
    try {
        val wm = context.applicationContext.getSystemService(Context.WIFI_SERVICE) as android.net.wifi.WifiManager
        val ipInt = wm.connectionInfo.ipAddress
        if (ipInt == 0) return "No Wi-Fi"
        return String.format(
            "%d.%d.%d.%d",
            ipInt and 0xff,
            ipInt shr 8 and 0xff,
            ipInt shr 16 and 0xff,
            ipInt shr 24 and 0xff
        )
    } catch (e: Exception) {
        return "Unknown IP"
    }
}
