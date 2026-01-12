package com.kodomo

import android.util.Log

class NativeClient {

    companion object {
        private const val TAG = "NativeClient"

        init {
            try {
                Log.d(TAG, "Loading kodomo-android library...")
                System.loadLibrary("kodomo-android")
                System.loadLibrary("SDL3")
                Log.d(TAG, "✓ kodomo-android library loaded successfully")
            } catch (e: UnsatisfiedLinkError) {
                Log.e(TAG, "Failed to load kodomo-android library", e)
                throw e
            }
        }
    }

    interface Callbacks {
        fun onNativeError(error: String)
        fun onNativeStatusChange(status: String)
        fun onNativeStatsUpdate(fps: Long, latency: Long)
    }

    private var callbacks: Callbacks? = null
    fun setCallbacks(callbacks: Callbacks) { this.callbacks = callbacks }

    @Suppress("unused")
    fun onNativeError(error: String) {
        Log.e(TAG, "Native error: $error")
        callbacks?.onNativeError(error)
    }

    @Suppress("unused")
    fun onNativeStatusChange(status: String) {
        Log.i(TAG, "Native status: $status")
        callbacks?.onNativeStatusChange(status)
    }

    @Suppress("unused")
    fun onNativeStatsUpdate(fps: Long, latency: Long) {
        callbacks?.onNativeStatsUpdate(fps, latency)
    }

    external fun nativeInit(serverAddress: String, width: Int, height: Int, useTailscale: Boolean, tailscaleHostname: String?): Boolean
    external fun nativeIsReady(): Boolean  // NEW: Check if SDL_AppInit completed
    external fun nativeConnect(): Boolean
    external fun nativeDisconnect()
    external fun nativeDestroy()
    external fun nativeTouchDown(x: Float, y: Float, fingerId: Int)
    external fun nativeTouchUp(x: Float, y: Float, fingerId: Int)
    external fun nativeTouchMove(x: Float, y: Float, fingerId: Int)
    external fun nativeOnPause()
    external fun nativeOnResume()
    external fun nativeOnLowMemory()
    external fun nativeGetStats(): String
}