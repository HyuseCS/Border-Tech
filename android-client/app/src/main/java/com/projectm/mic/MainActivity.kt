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
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat

// Design System Tokens (Cinema Mobile Style)
object SonusTheme {
    val BackgroundDeep = Color(0xFF020203)
    val BackgroundElevated = Color(0xFF0A0A0C)
    val Surface = Color(0x1AFFFFFF)
    val PrimaryAccent = Color(0xFF00FFCC) // Neon Cyan
    val SecondaryAccent = Color(0xFF5E6AD2) // Indigo
    val ErrorAccent = Color(0xFFFF3366) // Neon Magenta
    val TextMuted = Color(0xFF8A8F98)
    val Border = Color(0x14FFFFFF)
}

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme(
                colorScheme = darkColorScheme(
                    primary = SonusTheme.PrimaryAccent,
                    background = SonusTheme.BackgroundDeep,
                    surface = SonusTheme.BackgroundElevated
                )
            ) {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = SonusTheme.BackgroundDeep
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
    var isUsb by remember { mutableStateOf(sharedPref.getBoolean("is_usb", true)) }
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
    ) { perms ->
        hasMicPermission = perms[Manifest.permission.RECORD_AUDIO] == true
    }

    LaunchedEffect(Unit) {
        val permissions = mutableListOf(Manifest.permission.RECORD_AUDIO)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            permissions.add(Manifest.permission.POST_NOTIFICATIONS)
        }
        launcher.launch(permissions.toTypedArray())
    }

    // FIX: Re-fetch IP when state changes or mode changes
    LaunchedEffect(isRunning, isUsb) {
        if (isUsb) {
            localIp = "Localhost (USB)"
        } else {
            val ip = getLocalIpAddress(activity)
            localIp = if (ip == "0.0.0.0" || ip == "No Wi-Fi") "No Wi-Fi" else ip
        }
    }

    LaunchedEffect(portString, isUsb) {
        sharedPref.edit().apply {
            putString("port", portString)
            putBoolean("is_usb", isUsb)
            apply()
        }
    }

    val statusColor = when (connectionState) {
        AudioCaptureService.ConnectionState.CONNECTED -> SonusTheme.PrimaryAccent
        AudioCaptureService.ConnectionState.CONNECTING -> Color(0xFFFFCC00)
        AudioCaptureService.ConnectionState.ERROR -> SonusTheme.ErrorAccent
        AudioCaptureService.ConnectionState.DISCONNECTED -> SonusTheme.TextMuted
    }

    Box(modifier = Modifier.fillMaxSize()) {
        // Ambient background blobs for depth
        AmbientBlobs(statusColor)

        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(24.dp)
                .statusBarsPadding()
                .navigationBarsPadding(),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            // Header
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column {
                    Text(
                        text = "SONUS",
                        fontSize = 24.sp,
                        fontWeight = FontWeight.Black,
                        color = Color.White,
                        letterSpacing = 4.sp
                    )
                    Text(
                        text = "v1.0.0 Stable",
                        fontSize = 10.sp,
                        color = SonusTheme.TextMuted,
                        letterSpacing = 1.sp
                    )
                }
                Image(
                    painter = painterResource(id = R.drawable.ic_launcher_mic),
                    contentDescription = "Logo",
                    modifier = Modifier.size(40.dp)
                )
            }

            Spacer(modifier = Modifier.height(48.dp))

            // Main Status Card (Glassmorphism)
            GlassCard(
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(
                    modifier = Modifier.padding(24.dp),
                    horizontalAlignment = Alignment.CenterHorizontally
                ) {
                    Text(
                        text = when (connectionState) {
                            AudioCaptureService.ConnectionState.CONNECTED -> "SECURELY STREAMING"
                            AudioCaptureService.ConnectionState.CONNECTING -> "LISTENING FOR PC"
                            AudioCaptureService.ConnectionState.ERROR -> "SYSTEM ERROR"
                            AudioCaptureService.ConnectionState.DISCONNECTED -> if (isRunning) "WAITING..." else "SYSTEM READY"
                        },
                        color = statusColor,
                        fontSize = 12.sp,
                        fontWeight = FontWeight.Bold,
                        letterSpacing = 2.sp
                    )
                    
                    Spacer(modifier = Modifier.height(12.dp))
                    
                    Text(
                        text = if (isRunning || !isUsb) localIp else "OFFLINE",
                        color = Color.White,
                        fontSize = 28.sp,
                        fontWeight = FontWeight.Light,
                        textAlign = TextAlign.Center
                    )

                    if (isRunning || (!isUsb && localIp != "No Wi-Fi")) {
                        Spacer(modifier = Modifier.height(8.dp))
                        Text(
                            text = if (isUsb) "Run: adb reverse tcp:$portString tcp:$portString" else "Connect PC to $localIp:$portString",
                            color = SonusTheme.TextMuted,
                            fontSize = 11.sp,
                            textAlign = TextAlign.Center
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.weight(1f))

            // Mode Selector (Cinema Style)
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(56.dp)
                    .clip(RoundedCornerShape(28.dp))
                    .background(SonusTheme.BackgroundElevated)
                    .padding(4.dp),
                horizontalArrangement = Arrangement.SpaceEvenly
            ) {
                SelectorButton(
                    text = "USB (ADB)",
                    isSelected = isUsb,
                    onClick = { if (!isRunning) isUsb = true },
                    modifier = Modifier.weight(1f)
                )
                SelectorButton(
                    text = "WI-FI",
                    isSelected = !isUsb,
                    onClick = { if (!isRunning) isUsb = false },
                    modifier = Modifier.weight(1f)
                )
            }

            Spacer(modifier = Modifier.height(32.dp))

            // Power Control
            Box(contentAlignment = Alignment.Center) {
                if (isRunning && connectionState != AudioCaptureService.ConnectionState.ERROR) {
                    PowerPulse(statusColor)
                }

                val btnAnimateColor by animateColorAsState(
                    targetValue = if (isRunning) {
                        if (connectionState == AudioCaptureService.ConnectionState.ERROR) SonusTheme.ErrorAccent 
                        else statusColor
                    } else SonusTheme.BackgroundElevated,
                    animationSpec = tween(500),
                    label = "btn"
                )

                Surface(
                    modifier = Modifier
                        .size(120.dp)
                        .clip(CircleShape)
                        .clickable {
                            if (!hasMicPermission) {
                                Toast.makeText(activity, "Mic permission required", Toast.LENGTH_SHORT).show()
                                return@clickable
                            }
                            if (isRunning) {
                                AudioCaptureService.stopService(activity)
                            } else {
                                val port = portString.toIntOrNull() ?: 47999
                                AudioCaptureService.startService(activity, port, isUsb)
                            }
                        },
                    color = btnAnimateColor,
                    border = BorderStroke(4.dp, if (isRunning) Color.White.copy(alpha = 0.2f) else SonusTheme.Border),
                    shape = CircleShape,
                    shadowElevation = 8.dp
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        Text(
                            text = if (isRunning) "STOP" else "START",
                            color = if (isRunning) Color.White else SonusTheme.PrimaryAccent,
                            fontSize = 20.sp,
                            fontWeight = FontWeight.Black,
                            letterSpacing = 2.sp
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.weight(1f))

            // Settings Bottom Area
            Column(modifier = Modifier.fillMaxWidth()) {
                Text(
                    text = "PORT CONFIGURATION",
                    color = SonusTheme.TextMuted,
                    fontSize = 10.sp,
                    fontWeight = FontWeight.Bold,
                    letterSpacing = 1.sp,
                    modifier = Modifier.padding(start = 4.dp, bottom = 8.dp)
                )
                OutlinedTextField(
                    value = portString,
                    onValueChange = { if (it.length <= 5) portString = it },
                    placeholder = { Text("47999") },
                    singleLine = true,
                    enabled = !isRunning,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(16.dp),
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedBorderColor = SonusTheme.PrimaryAccent,
                        unfocusedBorderColor = SonusTheme.Border,
                        focusedTextColor = Color.White,
                        unfocusedTextColor = Color.White,
                        cursorColor = SonusTheme.PrimaryAccent,
                        focusedContainerColor = SonusTheme.BackgroundElevated,
                        unfocusedContainerColor = SonusTheme.BackgroundElevated
                    )
                )
            }
            
            Spacer(modifier = Modifier.height(16.dp))
            
            Text(
                text = "48kHz • 16-bit PCM • Mono • TLS 1.3",
                fontSize = 10.sp,
                color = SonusTheme.TextMuted.copy(alpha = 0.6f),
                modifier = Modifier.padding(bottom = 8.dp)
            )
        }
    }
}

@Composable
fun GlassCard(
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit
) {
    Surface(
        modifier = modifier,
        color = SonusTheme.Surface,
        shape = RoundedCornerShape(24.dp),
        border = BorderStroke(1.dp, SonusTheme.Border)
    ) {
        content()
    }
}

@Composable
fun SelectorButton(
    text: String,
    isSelected: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    val bgColor by animateColorAsState(
        targetValue = if (isSelected) SonusTheme.Surface else Color.Transparent,
        animationSpec = tween(300),
        label = "bg"
    )
    val textColor by animateColorAsState(
        targetValue = if (isSelected) Color.White else SonusTheme.TextMuted,
        animationSpec = tween(300),
        label = "text"
    )

    Box(
        modifier = modifier
            .fillMaxHeight()
            .clip(RoundedCornerShape(24.dp))
            .background(bgColor)
            .clickable(onClick = onClick),
        contentAlignment = Alignment.Center
    ) {
        Text(
            text = text,
            color = textColor,
            fontSize = 12.sp,
            fontWeight = if (isSelected) FontWeight.Bold else FontWeight.Medium,
            letterSpacing = 1.sp
        )
    }
}

@Composable
fun PowerPulse(color: Color) {
    val infiniteTransition = rememberInfiniteTransition(label = "pulse")
    val scale by infiniteTransition.animateFloat(
        initialValue = 1f,
        targetValue = 1.6f,
        animationSpec = infiniteRepeatable(
            animation = tween(2000, easing = LinearOutSlowInEasing),
            repeatMode = RepeatMode.Restart
        ),
        label = "scale"
    )
    val alpha by infiniteTransition.animateFloat(
        initialValue = 0.5f,
        targetValue = 0f,
        animationSpec = infiniteRepeatable(
            animation = tween(2000, easing = LinearOutSlowInEasing),
            repeatMode = RepeatMode.Restart
        ),
        label = "alpha"
    )
    
    Box(
        modifier = Modifier
            .size(120.dp)
            .scale(scale)
            .clip(CircleShape)
            .background(color.copy(alpha = alpha))
    )
}

@Composable
fun AmbientBlobs(statusColor: Color) {
    val infiniteTransition = rememberInfiniteTransition(label = "blobs")
    val offX by infiniteTransition.animateFloat(
        initialValue = -50f,
        targetValue = 50f,
        animationSpec = infiniteRepeatable(
            animation = tween(8000, easing = LinearOutSlowInEasing),
            repeatMode = RepeatMode.Reverse
        ),
        label = "x"
    )
    
    Box(modifier = Modifier.fillMaxSize().blur(80.dp)) {
        Box(
            modifier = Modifier
                .size(300.dp)
                .offset(x = offX.dp, y = (-100).dp)
                .background(SonusTheme.SecondaryAccent.copy(alpha = 0.15f), CircleShape)
        )
        Box(
            modifier = Modifier
                .size(250.dp)
                .align(Alignment.BottomEnd)
                .offset(x = (50 - offX).dp, y = 100.dp)
                .background(statusColor.copy(alpha = 0.1f), CircleShape)
        )
    }
}

fun getLocalIpAddress(context: Context): String {
    try {
        val interfaces = java.net.NetworkInterface.getNetworkInterfaces()
        while (interfaces.hasMoreElements()) {
            val networkInterface = interfaces.nextElement()
            // Ignore loopback, down, or virtual interfaces
            if (networkInterface.isLoopback || !networkInterface.isUp) continue
            
            val addresses = networkInterface.inetAddresses
            while (addresses.hasMoreElements()) {
                val addr = addresses.nextElement()
                // Only return IPv4 addresses
                if (!addr.isLoopbackAddress && addr is java.net.Inet4Address) {
                    return addr.hostAddress ?: continue
                }
            }
        }
        return "No Wi-Fi"
    } catch (e: Exception) {
        return "Unknown IP"
    }
}
