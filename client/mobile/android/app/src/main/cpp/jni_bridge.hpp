#pragma once

#include <jni.h>
#include <string>
#include <cstdint>

/**
 * Bridge between C++ and Java/Kotlin
 */
class JNIBridge {
public:
    static void set_jvm(JavaVM* jvm);
    static void set_activity(jobject activity);

    static JNIEnv* get_env();

    // Callbacks to Java
    static void call_java_error_callback(const std::string& error);
    static void call_java_status_callback(const std::string& status);
    static void call_java_stats_callback(uint64_t fps, uint64_t latency);
};