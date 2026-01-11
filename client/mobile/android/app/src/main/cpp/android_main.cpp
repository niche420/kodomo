#include <jni.h>
#include <android/log.h>
#include <string>
#include <memory>
#include <SDL3/SDL.h>
#include <SDL3/SDL_main.h>
#include "client_core.hpp"
#include "jni_bridge.hpp"

#define LOG_TAG "Kodomo"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

static std::unique_ptr<ClientCore> g_client;
static SDL_Window* g_window = nullptr;
static bool g_initialized = false;

struct AndroidInitParams {
    std::string server_address;
    bool use_tailscale = false;
    std::string tailscale_hostname;
    int width = 1920;
    int height = 1080;
};

static AndroidInitParams g_init_params;

PlatformCallbacks create_android_callbacks() {
    PlatformCallbacks callbacks;
    callbacks.on_error = [](const std::string& error) {
        LOGE("Error: %s", error.c_str());
        JNIBridge::call_java_error_callback(error);
    };
    callbacks.on_status_change = [](const std::string& status) {
        LOGI("Status: %s", status.c_str());
        JNIBridge::call_java_status_callback(status);
    };
    callbacks.on_stats_update = [](uint64_t fps, uint64_t latency) {
        JNIBridge::call_java_stats_callback(fps, latency);
    };
    callbacks.should_quit = []() { return false; };
    return callbacks;
}

SDL_AppResult SDL_AppInit(void** appstate, int argc, char* argv[]) {
    SDL_SetAppMetadata("Kodomo Client", "1.0", "com.kodomo");

    if (!SDL_Init(SDL_INIT_VIDEO)) {
        SDL_Log("SDL_Init failed: %s", SDL_GetError());
        return SDL_APP_FAILURE;
    }

    g_window = SDL_CreateWindow(
            "Kodomo",
            g_init_params.width,
            g_init_params.height,
            SDL_WINDOW_RESIZABLE | SDL_WINDOW_OPENGL
    );

    if (!g_window) {
        SDL_Log("SDL_CreateWindow failed: %s", SDL_GetError());
        return SDL_APP_FAILURE;
    }

    SDL_SetWindowFullscreen(g_window, true);

    ClientConfig config;
    config.server_address = g_init_params.server_address;
    config.width = g_init_params.width;
    config.height = g_init_params.height;
    config.fullscreen = true;
    config.use_tailscale = g_init_params.use_tailscale;
    config.tailscale_hostname = g_init_params.tailscale_hostname;
    config.is_mobile = true;
    config.touch_controls = true;
    config.dpi_scale = 1.0f;

    g_client = std::make_unique<ClientCore>();
    auto callbacks = create_android_callbacks();
    if (!g_client->initialize(config, callbacks, g_window)) {
        SDL_Log("ClientCore initialization failed");
        g_client.reset();
        return SDL_APP_FAILURE;
    }

    g_initialized = true;
    LOGI("SDL_AppInit completed successfully");
    return SDL_APP_CONTINUE;
}

SDL_AppResult SDL_AppIterate(void* appstate) {
    if (g_client && g_client->is_running()) {
        if (!g_client->update()) return SDL_APP_SUCCESS;
    }
    return SDL_APP_CONTINUE;
}

SDL_AppResult SDL_AppEvent(void* appstate, SDL_Event* event) {
    if (event->type == SDL_EVENT_QUIT) return SDL_APP_SUCCESS;
    if (g_client && g_client->is_running())
        g_client->handle_sdl_event(*event);
    return SDL_APP_CONTINUE;
}

void SDL_AppQuit(void* appstate, SDL_AppResult result) {
    if (g_client) g_client.reset();
    if (g_window) { SDL_DestroyWindow(g_window); g_window = nullptr; }
    g_initialized = false;
}

extern "C" {

JNIEXPORT jint JNI_OnLoad(JavaVM* vm, void* reserved) {
    JNIBridge::set_jvm(vm);
    LOGI("Native library loaded");
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
    if (!server_address) return JNI_FALSE;

    const char* addr = env->GetStringUTFChars(server_address, nullptr);
    g_init_params.server_address = addr;
    g_init_params.width = width;
    g_init_params.height = height;
    g_init_params.use_tailscale = use_tailscale;

    if (tailscale_hostname) {
        const char* ts = env->GetStringUTFChars(tailscale_hostname, nullptr);
        g_init_params.tailscale_hostname = ts;
        env->ReleaseStringUTFChars(tailscale_hostname, ts);
    }

    env->ReleaseStringUTFChars(server_address, addr);
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_com_kodomo_NativeClient_nativeConnect(JNIEnv*, jobject) {
    if (!g_client) return JNI_FALSE;
    return g_client->connect() ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jboolean JNICALL
Java_com_kodomo_NativeClient_nativeUpdate(JNIEnv*, jobject) {
    if (!g_client) return JNI_FALSE;
    return g_client->is_running() ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT void JNICALL
Java_com_kodomo_NativeClient_nativeDisconnect(JNIEnv*, jobject) {
    if (g_client) g_client->disconnect();
}

JNIEXPORT void JNICALL
Java_com_kodomo_NativeClient_nativeDestroy(JNIEnv*, jobject) {
    if (g_client) g_client.reset();
}

JNIEXPORT void JNICALL
Java_com_kodomo_NativeClient_nativeTouchDown(JNIEnv*, jobject, jfloat x, jfloat y, jint fingerId) {
    if (g_client) g_client->handle_touch_down(x, y, fingerId);
}

JNIEXPORT void JNICALL
Java_com_kodomo_NativeClient_nativeTouchUp(JNIEnv*, jobject, jfloat x, jfloat y, jint fingerId) {
    if (g_client) g_client->handle_touch_up(x, y, fingerId);
}

JNIEXPORT void JNICALL
Java_com_kodomo_NativeClient_nativeTouchMove(JNIEnv*, jobject, jfloat x, jfloat y, jint fingerId) {
    if (g_client) g_client->handle_touch_move(x, y, fingerId);
}

JNIEXPORT void JNICALL
Java_com_kodomo_NativeClient_nativeOnPause(JNIEnv*, jobject) { if (g_client) g_client->on_pause(); }

JNIEXPORT void JNICALL
Java_com_kodomo_NativeClient_nativeOnResume(JNIEnv*, jobject) { if (g_client) g_client->on_resume(); }

JNIEXPORT void JNICALL
Java_com_kodomo_NativeClient_nativeOnLowMemory(JNIEnv*, jobject) { if (g_client) g_client->on_low_memory(); }

JNIEXPORT void JNICALL
Java_com_kodomo_NativeClient_nativeHandleEvent(
        JNIEnv*, jobject,
        jint eventType, jfloat x, jfloat y)
{
    if (!g_client) return;
    switch(eventType) {
        case 0: g_client->handle_touch_down(x, y, 0); break;
        case 1: g_client->handle_touch_up(x, y, 0); break;
        case 2: g_client->handle_touch_move(x, y, 0); break;
        default: break;
    }
}

JNIEXPORT jstring JNICALL
Java_com_kodomo_NativeClient_nativeGetStats(JNIEnv* env, jobject) {
    if (!g_client) return env->NewStringUTF("{}");
    auto stats = g_client->get_stats();
    char buf[256];
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
