package com.kodomo

import android.os.Bundle
import android.util.Log
import android.view.WindowManager
import android.widget.Toast
import kotlinx.coroutines.*
import org.json.JSONObject
import org.libsdl.app.SDLActivity

class StreamingActivity : SDLActivity(), NativeClient.Callbacks {

    private val nativeClient = NativeClient()
    private var statsJob: Job? = null
    private var connectAttempted = false

    companion object {
        const val TAG = "StreamingActivity"
    }

    override fun getLibraries(): Array<String> {
        return arrayOf("kodomo-android")
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        Log.i(TAG, "╔════════════════════════════════════════╗")
        Log.i(TAG, "║     onCreate START                     ║")
        Log.i(TAG, "╚════════════════════════════════════════╝")

        nativeClient.setCallbacks(this)

        // Call super - this starts SDL and calls SDL_AppInit
        Log.i(TAG, "Calling super.onCreate() - SDL will start now")

        try {
            super.onCreate(savedInstanceState)
            Log.i(TAG, "✅ super.onCreate() returned")
        } catch (e: Exception) {
            Log.e(TAG, "💥 Exception in super.onCreate()", e)
            Toast.makeText(this, "SDL init error: ${e.message}", Toast.LENGTH_SHORT).show()
            finish()
            return
        }

        val serverAddress = intent.getStringExtra("server_address") ?: "127.0.0.1:8080"

        val width = resources.displayMetrics.widthPixels
        val height = resources.displayMetrics.heightPixels

        Log.i(TAG, "Server: $serverAddress")
        Log.i(TAG, "Resolution: ${width}x${height}")

        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)

        // Initialize native BEFORE calling super.onCreate()
        Log.i(TAG, "Calling nativeInit...")

        try {
            val success = nativeClient.nativeInit(serverAddress, width, height, false, null)

            if (!success) {
                Log.e(TAG, "❌ nativeInit failed!")
                Toast.makeText(this, "Failed to initialize client", Toast.LENGTH_SHORT).show()
                finish()
                return
            }

            Log.i(TAG, "✅ nativeInit succeeded")
        } catch (e: Exception) {
            Log.e(TAG, "💥 Exception in nativeInit", e)
            Toast.makeText(this, "Init error: ${e.message}", Toast.LENGTH_SHORT).show()
            finish()
            return
        }

        // Wait for SDL_AppInit to complete by polling nativeIsReady()
        CoroutineScope(Dispatchers.Main).launch {
            try {
                Log.i(TAG, "Waiting for SDL_AppInit to complete...")

                var ready = false
                var attempts = 0
                val maxAttempts = 100 // 10 seconds max

                while (!ready && attempts < maxAttempts && !isFinishing) {
                    ready = nativeClient.nativeIsReady()

                    if (!ready) {
                        if (attempts % 10 == 0) {
                            Log.i(TAG, "Still waiting for client initialization... (${attempts/10}s)")
                        }
                        delay(100)
                        attempts++
                    }
                }

                if (!ready) {
                    Log.e(TAG, "❌ Timeout waiting for client initialization!")
                    Toast.makeText(this@StreamingActivity,
                                 "Client initialization timeout",
                                 Toast.LENGTH_LONG).show()
                    finish()
                    return@launch
                }

                Log.i(TAG, "✅ Client is ready after ${attempts * 100}ms")

                // Now connect
                Log.i(TAG, "Attempting connection...")
                connectToServer()

            } catch (e: Exception) {
                Log.e(TAG, "💥 Exception in ready check", e)
                onNativeError("Ready check failed: ${e.message}")
            }
        }

        // Start stats monitoring
        statsJob = CoroutineScope(Dispatchers.Default).launch {
            delay(3000)

            while (isActive && !isFinishing) {
                try {
                    val stats = JSONObject(nativeClient.nativeGetStats())
                    val fps = stats.optDouble("fps", 0.0)
                    val latency = stats.optLong("latency", 0)
                    val received = stats.optLong("frames_received", 0)
                    val decoded = stats.optLong("frames_decoded", 0)
                    val rendered = stats.optLong("frames_rendered", 0)

                    if (received > 0 || decoded > 0 || rendered > 0) {
                        Log.i(TAG, "📊 FPS=$fps, Latency=${latency}ms, " +
                                   "RX=$received, Dec=$decoded, Render=$rendered")
                    } else if (connectAttempted) {
                        Log.d(TAG, "Connected, waiting for frames...")
                    }
                } catch (e: Exception) {
                    Log.w(TAG, "Stats error: ${e.message}")
                }
                delay(2000)
            }
        }

        Log.i(TAG, "╔════════════════════════════════════════╗")
        Log.i(TAG, "║     onCreate END                       ║")
        Log.i(TAG, "╚════════════════════════════════════════╝")
    }

    private suspend fun connectToServer() = withContext(Dispatchers.IO) {
        try {
            Log.i(TAG, "🔌 Calling nativeConnect()...")
            connectAttempted = true

            val connected = nativeClient.nativeConnect()

            withContext(Dispatchers.Main) {
                if (connected) {
                    Log.i(TAG, "✅ Connected successfully!")
                    Toast.makeText(this@StreamingActivity, "Connected!", Toast.LENGTH_SHORT).show()
                    onNativeStatusChange("Connected")
                } else {
                    Log.e(TAG, "❌ nativeConnect returned false")
                    Toast.makeText(this@StreamingActivity, "Connection failed - check server", Toast.LENGTH_LONG).show()
                    onNativeError("Failed to connect to server")

                    delay(2000)
                    if (!isFinishing) {
                        finish()
                    }
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "💥 Exception during connect", e)
            withContext(Dispatchers.Main) {
                Toast.makeText(this@StreamingActivity, "Error: ${e.message}", Toast.LENGTH_LONG).show()
                onNativeError("Connection exception: ${e.message}")

                delay(2000)
                if (!isFinishing) {
                    finish()
                }
            }
        }
    }

    override fun onPause() {
        Log.i(TAG, "onPause")
        try {
            nativeClient.nativeOnPause()
        } catch (e: Exception) {
            Log.e(TAG, "Error in onPause", e)
        }
        super.onPause()
    }

    override fun onResume() {
        super.onResume()
        Log.i(TAG, "onResume")
        try {
            nativeClient.nativeOnResume()
        } catch (e: Exception) {
            Log.e(TAG, "Error in onResume", e)
        }
    }

    override fun onLowMemory() {
        super.onLowMemory()
        Log.w(TAG, "⚠️ onLowMemory")
        try {
            nativeClient.nativeOnLowMemory()
        } catch (e: Exception) {
            Log.e(TAG, "Error in onLowMemory", e)
        }
    }

    override fun onDestroy() {
        Log.i(TAG, "onDestroy")

        try {
            statsJob?.cancel()
            nativeClient.nativeDisconnect()
            nativeClient.nativeDestroy()
        } catch (e: Exception) {
            Log.e(TAG, "Error in onDestroy", e)
        }

        super.onDestroy()
    }

    override fun onNativeError(error: String) {
        Log.e(TAG, "❌ Native error: $error")
        runOnUiThread {
            Toast.makeText(this, "Error: $error", Toast.LENGTH_LONG).show()
        }
    }

    override fun onNativeStatusChange(status: String) {
        Log.i(TAG, "ℹ️ Native status: $status")
    }

    override fun onNativeStatsUpdate(fps: Long, latency: Long) {
        // Handled in coroutine loop
    }
}