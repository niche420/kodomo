package com.kodomo

import android.app.Activity
import android.os.Bundle
import android.util.Log
import android.view.MotionEvent
import android.view.WindowManager
import kotlinx.coroutines.*
import org.json.JSONObject

class StreamingActivity : Activity(), NativeClient.Callbacks {

    private val nativeClient = NativeClient()
    private var statsJob: Job? = null

    private lateinit var serverAddress: String
    private var useTailscale = false
    private var tailscaleHostname: String? = null

    companion object { const val TAG = "StreamingActivity" }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Keep screen on
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)

        // Read parameters from intent
        serverAddress = intent.getStringExtra("server_address") ?: "127.0.0.1:8080"
        useTailscale = intent.getBooleanExtra("use_tailscale", false)
        tailscaleHostname = intent.getStringExtra("tailscale_hostname")

        // Set native callbacks
        nativeClient.setCallbacks(this)

        // Initialize native client on main thread
        CoroutineScope(Dispatchers.Main).launch {
            val success = nativeClient.nativeInit(
                serverAddress,
                resources.displayMetrics.widthPixels,
                resources.displayMetrics.heightPixels,
                useTailscale,
                tailscaleHostname
            )
            if (!success) {
                Log.e(TAG, "nativeInit failed")
                finish()
                return@launch
            }
        }

        // Connect in background
        CoroutineScope(Dispatchers.IO).launch {
            delay(500) // Give native side time to initialize
            val connected = nativeClient.nativeConnect()
            withContext(Dispatchers.Main) {
                if (!connected) {
                    Log.e(TAG, "nativeConnect failed")
                    finish()
                } else {
                    Log.i(TAG, "Connected to server")
                }
            }
        }

        // Start stats update loop
        statsJob = CoroutineScope(Dispatchers.Default).launch {
            while (isActive) {
                try {
                    val stats = JSONObject(nativeClient.nativeGetStats())
                    val fps = stats.optDouble("fps", 0.0)
                    val latency = stats.optLong("latency", 0)
                    Log.d(TAG, "Stats: FPS=$fps, Latency=${latency}ms")
                } catch (_: Exception) {}
                delay(1000)
            }
        }
    }

    // Forward Android touch events to native client
    override fun onTouchEvent(event: MotionEvent?): Boolean {
        event ?: return false
        val pointerId = event.getPointerId(0)
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> nativeClient.nativeTouchDown(event.x, event.y, pointerId)
            MotionEvent.ACTION_UP -> nativeClient.nativeTouchUp(event.x, event.y, pointerId)
            MotionEvent.ACTION_MOVE -> nativeClient.nativeTouchMove(event.x, event.y, pointerId)
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
        statsJob?.cancel()
        nativeClient.nativeDisconnect()
        nativeClient.nativeDestroy()
    }

    // NativeClient.Callbacks
    override fun onNativeError(error: String) {
        Log.e(TAG, "Native error: $error")
    }

    override fun onNativeStatusChange(status: String) {
        Log.i(TAG, "Native status: $status")
    }

    override fun onNativeStatsUpdate(fps: Long, latency: Long) {
        // Stats handled in loop, can ignore here
    }
}
