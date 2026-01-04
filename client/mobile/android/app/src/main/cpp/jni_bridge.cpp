#include "jni_bridge.hpp"
#include <android/log.h>

#define LOG_TAG "Kodomo-JNI"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)

static JavaVM* g_jvm = nullptr;
static jobject g_activity = nullptr;

void JNIBridge::set_jvm(JavaVM* jvm) {
    g_jvm = jvm;
}

void JNIBridge::set_activity(jobject activity) {
    g_activity = activity;
}

JNIEnv* JNIBridge::get_env() {
    if (!g_jvm) return nullptr;

    JNIEnv* env = nullptr;
    int status = g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);

    if (status == JNI_EDETACHED) {
        g_jvm->AttachCurrentThread(&env, nullptr);
    }

    return env;
}

void JNIBridge::call_java_error_callback(const std::string& error) {
    JNIEnv* env = get_env();
    if (!env || !g_activity) return;

    jclass cls = env->GetObjectClass(g_activity);
    jmethodID method = env->GetMethodID(cls, "onNativeError", "(Ljava/lang/String;)V");

    if (method) {
        jstring j_error = env->NewStringUTF(error.c_str());
        env->CallVoidMethod(g_activity, method, j_error);
        env->DeleteLocalRef(j_error);
    }

    env->DeleteLocalRef(cls);
}

void JNIBridge::call_java_status_callback(const std::string& status) {
    JNIEnv* env = get_env();
    if (!env || !g_activity) return;

    jclass cls = env->GetObjectClass(g_activity);
    jmethodID method = env->GetMethodID(cls, "onNativeStatusChange", "(Ljava/lang/String;)V");

    if (method) {
        jstring j_status = env->NewStringUTF(status.c_str());
        env->CallVoidMethod(g_activity, method, j_status);
        env->DeleteLocalRef(j_status);
    }

    env->DeleteLocalRef(cls);
}

void JNIBridge::call_java_stats_callback(uint64_t fps, uint64_t latency) {
    JNIEnv* env = get_env();
    if (!env || !g_activity) return;

    jclass cls = env->GetObjectClass(g_activity);
    jmethodID method = env->GetMethodID(cls, "onNativeStatsUpdate", "(JJ)V");

    if (method) {
        env->CallVoidMethod(g_activity, method, (jlong)fps, (jlong)latency);
    }

    env->DeleteLocalRef(cls);
}