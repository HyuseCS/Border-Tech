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
import androidx.compose.animation.core.animateFloatAsState
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
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
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
import androidx.compose.ui.graphics.Brush
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
    
    var ipAddress by remember { mutableStateOf(sharedPref.getString("ip_address", "192.168.1.100") ?: "192.168.1.100") }
    var portString by remember { mutableStateOf(sharedPref.getString("port", "47999") ?: "47999") }
    
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

    // Save preferences when IP/Port updates
    LaunchedEffect(ipAddress, portString) {
        sharedPref.edit().apply {
            putString("ip_address", ipAddress)
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

    val glowAlpha by animateFloatAsState(
        targetValue = if (connectionState == AudioCaptureService.ConnectionState.CONNECTED) 1.0f else 0.4f,
        animationSpec = tween(1000),
        label = "glow"
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
                text = "Hi-Fi Wireless Audio Link",
                fontSize = 14.sp,
                color = Color(0xFF8E8E9F),
                fontFamily = FontFamily.SansSerif
            )
        }

        // Connection Status Box
        Card(
            modifier = Modifier
                .fillMaxWidth()
                .border(1.dp, neonColor.copy(alpha = glowAlpha), RoundedCornerShape(16.dp)),
            colors = CardDefaults.cardColors(containerColor = Color(0xFF16161D)),
            shape = RoundedCornerShape(16.dp)
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(20.dp),
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.Center
                ) {
                    Box(
                        modifier = Modifier
                            .size(12.dp)
                            .clip(CircleShape)
                            .background(neonColor)
                    )
                    Spacer(modifier = Modifier.width(10.dp))
                    Text(
                        text = connectionState.name,
                        fontSize = 16.sp,
                        fontWeight = FontWeight.Bold,
                        color = Color.White
                    )
                }

                if (connectionState == AudioCaptureService.ConnectionState.ERROR && errorMessage.isNotEmpty()) {
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = errorMessage,
                        fontSize = 12.sp,
                        color = Color(0xFFFF3366),
                        textAlign = TextAlign.Center
                    )
                }
            }
        }

        // Main Controller Area
        Box(
            contentAlignment = Alignment.Center,
            modifier = Modifier
                .size(200.dp)
                .scale(if (isRunning) 1.05f else 1.0f)
        ) {
            // Pulsing background rings
            Box(
                modifier = Modifier
                    .size(190.dp)
                    .clip(CircleShape)
                    .border(2.dp, neonColor.copy(alpha = 0.2f), CircleShape)
            )
            Box(
                modifier = Modifier
                    .size(170.dp)
                    .clip(CircleShape)
                    .border(1.dp, neonColor.copy(alpha = 0.4f), CircleShape)
            )

            // Actual interactive button
            val btnColor by animateColorAsState(
                targetValue = if (isRunning) Color(0xFFFF3366) else Color(0xFF00FFCC),
                animationSpec = tween(500),
                label = "btnColor"
            )

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
                        AudioCaptureService.startService(activity, ipAddress, port)
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

        // Input Fields (IP & Port)
        Column(modifier = Modifier.fillMaxWidth()) {
            OutlinedTextField(
                value = ipAddress,
                onValueChange = { ipAddress = it },
                label = { Text("Receiver PC IP") },
                singleLine = true,
                enabled = !isRunning,
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

            Spacer(modifier = Modifier.height(12.dp))

            OutlinedTextField(
                value = portString,
                onValueChange = { portString = it },
                label = { Text("Port") },
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
