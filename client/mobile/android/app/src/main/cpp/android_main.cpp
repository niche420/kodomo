#include <jni.h>
#include <android/log.h>
#include <string>
#include <memory>
#define SDL_MAIN_USE_CALLBACKS
#include <SDL3/SDL.h>
#include <SDL3/SDL_main.h>
#include "client_core.hpp"
#include "jni_bridge.hpp"

#define LOG_TAG "Kodomo"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

// Global state
static std::unique_ptr<ClientCore> g_client;
static SDL_Window* g_window = nullptr;
static bool g_initialized = false;

// Init params - set by Java BEFORE SDL starts
struct InitParams {
    std::string server_address;
    int width = 1920;
    int height = 1080;
    bool use_tailscale = false;
    std::string tailscale_hostname;
    bool ready = false;
} g_params;

PlatformCallbacks create_callbacks() {
    PlatformCallbacks callbacks;
    callbacks.on_error = [](const std::string& error) {
        LOGE("❌ Error: %s", error.c_str());
        JNIBridge::call_java_error_callback(error);
    };
    callbacks.on_status_change = [](const std::string& status) {
        LOGI("ℹ️ Status: %s", status.c_str());
        JNIBridge::call_java_status_callback(status);
    };
    callbacks.on_stats_update = [](uint64_t fps, uint64_t latency) {
        JNIBridge::call_java_stats_callback(fps, latency);
    };
    callbacks.should_quit = []() { return false; };
    return callbacks;
}

// Called by SDL on its own thread when activity starts
SDL_AppResult SDL_AppInit(void** appstate, int argc, char* argv[]) {
    LOGI("╔════════════════════════════════════════╗");
    LOGI("║      SDL_AppInit STARTING              ║");
    LOGI("╚════════════════════════════════════════╝");

    SDL_SetAppMetadata("Kodomo Client", "1.0", "com.kodomo");

    // Wait for Java to set params (max 10 seconds)
    LOGI("Waiting for init params from Java...");
    int wait_ms = 0;
    while (!g_params.ready && wait_ms < 10000) {
        SDL_Delay(100);
        wait_ms += 100;

        if (wait_ms % 1000 == 0) {
            LOGI("Still waiting... (%d seconds)", wait_ms / 1000);
        }
    }

    if (!g_params.ready) {
        LOGE("❌ TIMEOUT: Java didn't set init params after 10 seconds!");
        return SDL_APP_FAILURE;
    }

    LOGI("✅ Got params from Java:");
    LOGI("   Server: %s", g_params.server_address.c_str());
    LOGI("   Resolution: %dx%d", g_params.width, g_params.height);
    LOGI("   Tailscale: %s", g_params.use_tailscale ? "enabled" : "disabled");

    // Initialize SDL
    LOGI("Initializing SDL...");
    if (!SDL_Init(SDL_INIT_VIDEO | SDL_INIT_EVENTS)) {
        LOGE("❌ SDL_Init failed: %s", SDL_GetError());
        return SDL_APP_FAILURE;
    }
    LOGI("✅ SDL initialized");

    // Create window
    LOGI("Creating window...");
    g_window = SDL_CreateWindow(
        "Kodomo",
        g_params.width,
        g_params.height,
        SDL_WINDOW_RESIZABLE | SDL_WINDOW_OPENGL | SDL_WINDOW_HIGH_PIXEL_DENSITY
    );

    if (!g_window) {
        LOGE("❌ SDL_CreateWindow failed: %s", SDL_GetError());
        return SDL_APP_FAILURE;
    }
    LOGI("✅ Window created: %dx%d", g_params.width, g_params.height);

    // Set fullscreen
    if (!SDL_SetWindowFullscreen(g_window, true)) {
        LOGE("⚠️ Failed to set fullscreen: %s", SDL_GetError());
    } else {
        LOGI("✅ Fullscreen enabled");
    }

    // Initialize ClientCore
    LOGI("Initializing ClientCore...");

    ClientConfig config;
    config.server_address = g_params.server_address;
    config.width = g_params.width;
    config.height = g_params.height;
    config.fullscreen = true;
    config.use_tailscale = g_params.use_tailscale;
    config.tailscale_hostname = g_params.tailscale_hostname;
    config.is_mobile = true;
    config.touch_controls = true;
    config.dpi_scale = 1.0f;

    g_client = std::make_unique<ClientCore>();
    auto callbacks = create_callbacks();

    if (!g_client->initialize(config, callbacks, g_window)) {
        LOGE("❌ ClientCore initialization FAILED");
        g_client.reset();
        return SDL_APP_FAILURE;
    }
    LOGI("✅ ClientCore initialized");

    // CRITICAL: Set this flag so Java knows we're ready
    g_initialized = true;

    LOGI("╔════════════════════════════════════════╗");
    LOGI("║   SDL_AppInit COMPLETE - Ready!        ║");
    LOGI("╚════════════════════════════════════════╝");

    return SDL_APP_CONTINUE;
}

// Called every frame by SDL
SDL_AppResult SDL_AppIterate(void* appstate) {
    if (!g_initialized || !g_client) {
        return SDL_APP_CONTINUE;
    }

    // Update client
    if (!g_client->is_connected()) {
        LOGI("Client not connected yet");
        return SDL_APP_CONTINUE;
    }

    if (!g_client->update()) {
        LOGI("Client update returned false - exiting");
        return SDL_APP_SUCCESS;
    }

    return SDL_APP_CONTINUE;
}

// Called for each SDL event
SDL_AppResult SDL_AppEvent(void* appstate, SDL_Event* event) {
    if (event->type == SDL_EVENT_QUIT) {
        LOGI("Received SDL_EVENT_QUIT");
        return SDL_APP_SUCCESS;
    }

    if (g_initialized && g_client) {
        g_client->handle_sdl_event(*event);
    }

    return SDL_APP_CONTINUE;
}

// Called when app is quitting
void SDL_AppQuit(void* appstate, SDL_AppResult result) {
    LOGI("╔════════════════════════════════════════╗");
    LOGI("║         SDL_AppQuit - Cleanup          ║");
    LOGI("╚════════════════════════════════════════╝");

    if (g_client) {
        g_client->disconnect();
        g_client.reset();
        LOGI("✅ Client destroyed");
    }

    if (g_window) {
        SDL_DestroyWindow(g_window);
        g_window = nullptr;
        LOGI("✅ Window destroyed");
    }

    g_initialized = false;
    LOGI("SDL_AppQuit complete");
}

// ========== JNI FUNCTIONS ==========

extern "C" {

JNIEXPORT jint JNI_OnLoad(JavaVM* vm, void* reserved) {
    JNIBridge::set_jvm(vm);
    LOGI("JNI_OnLoad: Native library loaded");
    return JNI_VERSION_1_6;
}

JNIEXPORT jboolean JNICALL
Java_com_kodomo_NativeClient_nativeInit(
        JNIEnv* env, jobject thiz,
        jstring server_address,
        jint width, jint height,
        jboolean use_tailscale,
        jstring tailscale_hostname
) {
    LOGI("╔════════════════════════════════════════╗");
    LOGI("║        nativeInit CALLED               ║");
    LOGI("╚════════════════════════════════════════╝");

    if (!server_address) {
        LOGE("❌ server_address is null!");
        return JNI_FALSE;
    }

    const char* addr = env->GetStringUTFChars(server_address, nullptr);
    g_params.server_address = addr;
    env->ReleaseStringUTFChars(server_address, addr);

    g_params.width = width;
    g_params.height = height;
    g_params.use_tailscale = use_tailscale;

    if (tailscale_hostname) {
        const char* ts = env->GetStringUTFChars(tailscale_hostname, nullptr);
        g_params.tailscale_hostname = ts;
        env->ReleaseStringUTFChars(tailscale_hostname, ts);
    }

    // Signal to SDL_AppInit that params are ready
    g_params.ready = true;

    LOGI("✅ Params set and ready:");
    LOGI("   Server: %s", g_params.server_address.c_str());
    LOGI("   Size: %dx%d", width, height);

    return JNI_TRUE;
}

// NEW: Check if client is ready
JNIEXPORT jboolean JNICALL
Java_com_kodomo_NativeClient_nativeIsReady(JNIEnv*, jobject) {
    return g_initialized ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jboolean JNICALL
Java_com_kodomo_NativeClient_nativeConnect(JNIEnv*, jobject) {
    LOGI("╔════════════════════════════════════════╗");
    LOGI("║       nativeConnect CALLED             ║");
    LOGI("╚════════════════════════════════════════╝");

    if (!g_initialized) {
        LOGE("❌ nativeConnect: Client not initialized yet!");
        return JNI_FALSE;
    }

    if (!g_client) {
        LOGE("❌ nativeConnect: g_client is null!");
        return JNI_FALSE;
    }

    LOGI("Calling client->connect()...");
    bool success = g_client->connect();

    if (success) {
        LOGI("✅ Connection successful!");
    } else {
        LOGE("❌ Connection failed!");
    }

    return success ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jboolean JNICALL
Java_com_kodomo_NativeClient_nativeUpdate(JNIEnv*, jobject) {
    return (g_client && g_initialized && g_client->is_running()) ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT void JNICALL
Java_com_kodomo_NativeClient_nativeDisconnect(JNIEnv*, jobject) {
    LOGI("nativeDisconnect");
    if (g_client) g_client->disconnect();
}

JNIEXPORT void JNICALL
Java_com_kodomo_NativeClient_nativeDestroy(JNIEnv*, jobject) {
    LOGI("nativeDestroy");
    if (g_client) {
        g_client->disconnect();
        g_client.reset();
    }
    g_initialized = false;
}

JNIEXPORT void JNICALL
Java_com_kodomo_NativeClient_nativeTouchDown(JNIEnv*, jobject, jfloat x, jfloat y, jint id) {
    if (g_client && g_initialized) g_client->handle_touch_down(x, y, id);
}

JNIEXPORT void JNICALL
Java_com_kodomo_NativeClient_nativeTouchUp(JNIEnv*, jobject, jfloat x, jfloat y, jint id) {
    if (g_client && g_initialized) g_client->handle_touch_up(x, y, id);
}

JNIEXPORT void JNICALL
Java_com_kodomo_NativeClient_nativeTouchMove(JNIEnv*, jobject, jfloat x, jfloat y, jint id) {
    if (g_client && g_initialized) g_client->handle_touch_move(x, y, id);
}

JNIEXPORT void JNICALL
Java_com_kodomo_NativeClient_nativeOnPause(JNIEnv*, jobject) {
    LOGI("nativeOnPause");
    if (g_client && g_initialized) g_client->on_pause();
}

JNIEXPORT void JNICALL
Java_com_kodomo_NativeClient_nativeOnResume(JNIEnv*, jobject) {
    LOGI("nativeOnResume");
    if (g_client && g_initialized) g_client->on_resume();
}

JNIEXPORT void JNICALL
Java_com_kodomo_NativeClient_nativeOnLowMemory(JNIEnv*, jobject) {
    if (g_client && g_initialized) g_client->on_low_memory();
}

JNIEXPORT jstring JNICALL
Java_com_kodomo_NativeClient_nativeGetStats(JNIEnv* env, jobject) {
    if (!g_client || !g_initialized) {
        return env->NewStringUTF("{\"fps\":0,\"latency\":0,\"frames_received\":0,\"frames_decoded\":0,\"frames_rendered\":0}");
    }

    auto stats = g_client->get_stats();
    char buf[512];
    snprintf(buf, sizeof(buf),
             "{\"fps\":%.1f,\"latency\":%llu,\"frames_received\":%llu,\"frames_decoded\":%llu,\"frames_rendered\":%llu}",
             stats.average_fps,
             (unsigned long long)stats.average_latency_ms,
             (unsigned long long)stats.frames_received,
             (unsigned long long)stats.frames_decoded,
             (unsigned long long)stats.frames_rendered
    );
    return env->NewStringUTF(buf);
}

} // extern "C"