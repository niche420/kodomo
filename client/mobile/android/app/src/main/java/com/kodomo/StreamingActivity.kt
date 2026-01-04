package com.kodomo

import android.R.attr.height
import android.R.attr.width
import android.app.Activity
import android.os.Bundle
import android.util.Log
import android.view.MotionEvent
import android.view.View
import android.view.WindowManager
import android.widget.Toast
import kotlinx.coroutines.*
import org.json.JSONObject

class StreamingActivity : Activity() {

    private val nativeClient = NativeClient()
    private var updateJob: Job? = null
    private var statsJob: Job? = null

    private lateinit var serverAddress: String
    private var useTailscale: Boolean = false
    private var tailscaleHostname: String? = null

    companion object {
        const val TAG = "StreamingActivity"
        const val EXTRA_SERVER_ADDRESS = "server_address"
        const val EXTRA_USE_TAILSCALE = "use_tailscale"
        const val EXTRA_TAILSCALE_HOSTNAME = "tailscale_hostname"
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Keep screen on
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)

        // Fullscreen immersive mode
        window.decorView.systemUiVisibility = (
            View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
            or View.SYSTEM_UI_FLAG_FULLSCREEN
            or View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
            or View.SYSTEM_UI_FLAG_LAYOUT_STABLE
            or View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
            or View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
        )

        // Get connection parameters
        serverAddress = intent.getStringExtra(EXTRA_SERVER_ADDRESS) ?: "127.0.0.1:8080"
        useTailscale = intent.getBooleanExtra(EXTRA_USE_TAILSCALE, false)
        tailscaleHostname = intent.getStringExtra(EXTRA_TAILSCALE_HOSTNAME)

        // SDL will create its own view
        // Initialize native client
        initializeClient()
    }

    private fun initializeClient() {
        val metrics = resources.displayMetrics
        val width = metrics.widthPixels
        val height = metrics.heightPixels

        Log.i(TAG, "Initializing client: ${width}x${height}")
        Log.i(TAG, "Server: $serverAddress")
        Log.i(TAG, "Tailscale: $useTailscale")

        val success = nativeClient.nativeInit(
            serverAddress,
            width,
            height,
            useTailscale,
            tailscaleHostname
        )

        if (!success) {
            showError("Failed to initialize client")
            finish()
            return
        }

        // Connect in background
        CoroutineScope(Dispatchers.IO).launch {
            val connected = nativeClient.nativeConnect()

            withContext(Dispatchers.Main) {
                if (connected) {
                    onConnected()
                } else {
                    showError("Failed to connect to server")
                    finish()
                }
            }
        }
    }

    private fun onConnected() {
        Log.i(TAG, "Connected to server")
        Toast.makeText(this, "Connected", Toast.LENGTH_SHORT).show()

        // Start update loop
        startUpdateLoop()

        // Start stats display
        startStatsLoop()
    }

    private fun startUpdateLoop() {
        updateJob = CoroutineScope(Dispatchers.Default).launch {
            while (isActive) {
                val shouldContinue = nativeClient.nativeUpdate()

                if (!shouldContinue) {
                    withContext(Dispatchers.Main) {
                        onDisconnected()
                    }
                    break
                }

                // ~60Hz update rate
                delay(16)
            }
        }
    }

    private fun startStatsLoop() {
        statsJob = CoroutineScope(Dispatchers.Default).launch {
            while (isActive) {
                try {
                    val statsJson = nativeClient.nativeGetStats()
                    val stats = JSONObject(statsJson)

                    val fps = stats.optDouble("fps", 0.0)
                    val latency = stats.optLong("latency", 0)

                    Log.d(TAG, "Stats: FPS=$fps, Latency=${latency}ms")

                } catch (e: Exception) {
                    Log.e(TAG, "Failed to parse stats", e)
                }

                delay(1000) // Update every second
            }
        }
    }

    private fun onDisconnected() {
        Log.i(TAG, "Disconnected from server")
        Toast.makeText(this, "Disconnected", Toast.LENGTH_SHORT).show()
        finish()
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        val action = event.actionMasked
        val pointerIndex = event.actionIndex
        val pointerId = event.getPointerId(pointerIndex)

        // Normalize coordinates to 0-1
        val x = event.getX(pointerIndex) / width.toFloat()
        val y = event.getY(pointerIndex) / height.toFloat()

        when (action) {
            MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN -> {
                nativeClient.nativeTouchDown(x, y, pointerId)
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP -> {
                nativeClient.nativeTouchUp(x, y, pointerId)
            }
            MotionEvent.ACTION_MOVE -> {
                // Handle all active pointers
                for (i in 0 until event.pointerCount) {
                    val id = event.getPointerId(i)
                    val px = event.getX(i) / width.toFloat()
                    val py = event.getY(i) / height.toFloat()
                    nativeClient.nativeTouchMove(px, py, id)
                }
            }
        }

        return true
    }

    override fun onPause() {
        super.onPause()
        nativeClient.nativeOnPause()
    }

    override fun onResume() {
        super.onResume()
        nativeClient.nativeOnResume()
    }

    override fun onLowMemory() {
        super.onLowMemory()
        nativeClient.nativeOnLowMemory()
    }

    override fun onDestroy() {
        super.onDestroy()

        // Stop loops
        updateJob?.cancel()
        statsJob?.cancel()

        // Cleanup native
        nativeClient.nativeDisconnect()
        nativeClient.nativeDestroy()
    }

    // Callbacks from native code
    @Suppress("unused")
    fun onNativeError(error: String) {
        runOnUiThread {
            showError(error)
        }
    }

    @Suppress("unused")
    fun onNativeStatusChange(status: String) {
        runOnUiThread {
            Log.i(TAG, "Status: $status")
        }
    }

    @Suppress("unused")
    fun onNativeStatsUpdate(fps: Long, latency: Long) {
        // Stats handled in statsJob
    }

    private fun showError(message: String) {
        Log.e(TAG, "Error: $message")
        Toast.makeText(this, message, Toast.LENGTH_LONG).show()
    }
}