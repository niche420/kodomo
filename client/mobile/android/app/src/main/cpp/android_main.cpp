#include <jni.h>
#include <android/log.h>
#include <SDL3/SDL.h>
#include <string>
#include <memory>

#include "client_core.hpp"
#include "jni_bridge.hpp"

#define LOG_TAG "Kodomo"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

// Global client instance
static std::unique_ptr<ClientCore> g_client;
static JavaVM* g_jvm = nullptr;
static jobject g_activity = nullptr;

// Platform callbacks for Android
PlatformCallbacks create_android_callbacks() {
    PlatformCallbacks callbacks;

    callbacks.on_error = [](const std::string& error) {
        LOGE("Error: %s", error.c_str());
        // TODO: Call back to Java layer
        JNIBridge::call_java_error_callback(error);
    };

    callbacks.on_status_change = [](const std::string& status) {
        LOGI("Status: %s", status.c_str());
        JNIBridge::call_java_status_callback(status);
    };

    callbacks.on_stats_update = [](uint64_t fps, uint64_t latency) {
        JNIBridge::call_java_stats_callback(fps, latency);
    };

    callbacks.should_quit = []() {
        // On Android, we don't quit via this mechanism
        return false;
    };

    return callbacks;
}

extern "C" {

// JNI_OnLoad - called when library is loaded
JNIEXPORT jint JNI_OnLoad(JavaVM* vm, void* reserved) {
    LOGI("Kodomo native library loaded");
    g_jvm = vm;
    JNIBridge::set_jvm(vm);
    return JNI_VERSION_1_6;
}

// Initialize the client
JNIEXPORT jboolean JNICALL
Java_com_kodomo_client_NativeClient_nativeInit(
    JNIEnv* env,
    jobject thiz,
    jstring server_address,
    jint width,
    jint height,
    jboolean use_tailscale,
    jstring tailscale_hostname
) {
    LOGI("nativeInit called");

    // Store activity reference
    g_activity = env->NewGlobalRef(thiz);
    JNIBridge::set_activity(g_activity);

    // Convert Java strings to C++
    const char* server_addr_cstr = env->GetStringUTFChars(server_address, nullptr);
    std::string server_addr(server_addr_cstr);
    env->ReleaseStringUTFChars(server_address, server_addr_cstr);

    std::string ts_hostname;
    if (tailscale_hostname != nullptr) {
        const char* ts_cstr = env->GetStringUTFChars(tailscale_hostname, nullptr);
        ts_hostname = std::string(ts_cstr);
        env->ReleaseStringUTFChars(tailscale_hostname, ts_cstr);
    }

    // Create config
    ClientConfig config;
    config.server_address = server_addr;
    config.width = width;
    config.height = height;
    config.fullscreen = false;
    config.use_tailscale = use_tailscale;
    config.tailscale_hostname = ts_hostname;
    config.is_mobile = true;
    config.touch_controls = true;

    // Get DPI scale
    float dpi = 160.0f; // Default

    // Create client
    g_client = std::make_unique<ClientCore>();

    // Initialize
    auto callbacks = create_android_callbacks();
    if (!g_client->initialize(config, callbacks, nullptr)) {
        LOGE("Failed to initialize client core");
        g_client.reset();
        return JNI_FALSE;
    }

    LOGI("Client initialized successfully");
    return JNI_TRUE;
}

// Connect to server
JNIEXPORT jboolean JNICALL
Java_com_kodomo_client_NativeClient_nativeConnect(JNIEnv* env, jobject thiz) {
    if (!g_client) {
        LOGE("Client not initialized");
        return JNI_FALSE;
    }

    LOGI("Connecting to server...");
    bool success = g_client->connect();

    if (success) {
        LOGI("Connected successfully");
    } else {
        LOGE("Connection failed");
    }

    return success ? JNI_TRUE : JNI_FALSE;
}

// Main update loop - called from Java at ~60Hz
JNIEXPORT jboolean JNICALL
Java_com_kodomo_client_NativeClient_nativeUpdate(JNIEnv* env, jobject thiz) {
    if (!g_client) {
        return JNI_FALSE;
    }

    return g_client->update() ? JNI_TRUE : JNI_FALSE;
}

// Disconnect
JNIEXPORT void JNICALL
Java_com_kodomo_client_NativeClient_nativeDisconnect(JNIEnv* env, jobject thiz) {
    if (g_client) {
        LOGI("Disconnecting...");
        g_client->disconnect();
    }
}

// Cleanup
JNIEXPORT void JNICALL
Java_com_kodomo_client_NativeClient_nativeDestroy(JNIEnv* env, jobject thiz) {
    LOGI("Destroying client");

    if (g_client) {
        g_client.reset();
    }

    if (g_activity) {
        env->DeleteGlobalRef(g_activity);
        g_activity = nullptr;
    }
}

// Handle SDL events
JNIEXPORT void JNICALL
Java_com_kodomo_client_NativeClient_nativeHandleEvent(
    JNIEnv* env,
    jobject thiz,
    jint event_type,
    jfloat x,
    jfloat y
) {
    if (!g_client) return;

    SDL_Event event;
    SDL_zero(event);

    event.type = event_type;

    // Convert event based on type
    // (SDL will handle most events internally)
    g_client->handle_sdl_event(event);
}

// Touch events
JNIEXPORT void JNICALL
Java_com_kodomo_client_NativeClient_nativeTouchDown(
    JNIEnv* env,
    jobject thiz,
    jfloat x,
    jfloat y,
    jint finger_id
) {
    if (g_client) {
        g_client->handle_touch_down(x, y, finger_id);
    }
}

JNIEXPORT void JNICALL
Java_com_kodomo_client_NativeClient_nativeTouchUp(
    JNIEnv* env,
    jobject thiz,
    jfloat x,
    jfloat y,
    jint finger_id
) {
    if (g_client) {
        g_client->handle_touch_up(x, y, finger_id);
    }
}

JNIEXPORT void JNICALL
Java_com_kodomo_client_NativeClient_nativeTouchMove(
    JNIEnv* env,
    jobject thiz,
    jfloat x,
    jfloat y,
    jint finger_id
) {
    if (g_client) {
        g_client->handle_touch_move(x, y, finger_id);
    }
}

// Lifecycle
JNIEXPORT void JNICALL
Java_com_kodomo_client_NativeClient_nativeOnPause(JNIEnv* env, jobject thiz) {
    if (g_client) {
        g_client->on_pause();
    }
}

JNIEXPORT void JNICALL
Java_com_kodomo_client_NativeClient_nativeOnResume(JNIEnv* env, jobject thiz) {
    if (g_client) {
        g_client->on_resume();
    }
}

JNIEXPORT void JNICALL
Java_com_kodomo_client_NativeClient_nativeOnLowMemory(JNIEnv* env, jobject thiz) {
    if (g_client) {
        g_client->on_low_memory();
    }
}

// Get stats
JNIEXPORT jstring JNICALL
Java_com_kodomo_client_NativeClient_nativeGetStats(JNIEnv* env, jobject thiz) {
    if (!g_client) {
        return env->NewStringUTF("{}");
    }

    auto stats = g_client->get_stats();

    // Create JSON string
    char buffer[256];
    snprintf(buffer, sizeof(buffer),
        "{\"fps\":%.1f,\"latency\":%llu,\"frames_received\":%llu,\"frames_decoded\":%llu,\"frames_rendered\":%llu}",
        stats.average_fps,
        (unsigned long long)stats.average_latency_ms,
        (unsigned long long)stats.frames_received,
        (unsigned long long)stats.frames_decoded,
        (unsigned long long)stats.frames_rendered
    );

    return env->NewStringUTF(buffer);
}

} // extern "C"