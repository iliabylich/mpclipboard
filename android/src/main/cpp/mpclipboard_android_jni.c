#include <jni.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "bindings.h"

void mpclipboard_setup_rustls_on_jvm(JNIEnv *env, jobject context);

static void throw_java_exception(JNIEnv *env, const char *class_name, const char *message) {
    if ((*env)->ExceptionCheck(env)) {
        return;
    }

    jclass cls = (*env)->FindClass(env, class_name);
    if (cls == NULL) {
        return;
    }

    (*env)->ThrowNew(env, cls, message);
}

static void throw_runtime_exception(JNIEnv *env, const char *message) {
    throw_java_exception(env, "java/lang/RuntimeException", message);
}

static void throw_out_of_memory_error(JNIEnv *env, const char *message) {
    throw_java_exception(env, "java/lang/OutOfMemoryError", message);
}

static char *copy_bytes_as_c_string(JNIEnv *env, jbyteArray array) {
    if (array == NULL) {
        throw_runtime_exception(env, "byte array argument must not be null");
        return NULL;
    }

    jsize len = (*env)->GetArrayLength(env, array);
    char *buffer = calloc((size_t) len + 1U, sizeof(char));
    if (buffer == NULL) {
        throw_out_of_memory_error(env, "failed to allocate string buffer");
        return NULL;
    }

    (*env)->GetByteArrayRegion(env, array, 0, len, (jbyte *) buffer);
    if ((*env)->ExceptionCheck(env)) {
        free(buffer);
        return NULL;
    }

    buffer[len] = '\0';
    return buffer;
}

static jobject new_output(JNIEnv *env, jint tag, jint connectivity, jbyteArray text) {
    jclass cls = (*env)->FindClass(env, "dev/mpclipboard/android/NativeOutput");
    if (cls == NULL) {
        return NULL;
    }

    jmethodID ctor = (*env)->GetMethodID(env, cls, "<init>", "(II[B)V");
    if (ctor == NULL) {
        return NULL;
    }

    return (*env)->NewObject(env, cls, ctor, tag, connectivity, text);
}

JNIEXPORT jboolean JNICALL
Java_dev_mpclipboard_android_Ffi_mpclipboard_1init(JNIEnv *env, jclass clazz) {
    (void) env;
    (void) clazz;
    return mpclipboard_init();
}

JNIEXPORT void JNICALL
Java_dev_mpclipboard_android_Ffi_mpclipboard_1setup_1rustls_1on_1jvm(
    JNIEnv *env,
    jclass clazz,
    jobject context
) {
    (void) clazz;
    mpclipboard_setup_rustls_on_jvm(env, context);
}

JNIEXPORT jlong JNICALL
Java_dev_mpclipboard_android_Ffi_mpclipboard_1config_1new(
    JNIEnv *env,
    jclass clazz,
    jbyteArray uri,
    jbyteArray token,
    jbyteArray name
) {
    (void) clazz;

    char *uri_bytes = copy_bytes_as_c_string(env, uri);
    char *token_bytes = copy_bytes_as_c_string(env, token);
    char *name_bytes = copy_bytes_as_c_string(env, name);

    if (uri_bytes == NULL || token_bytes == NULL || name_bytes == NULL) {
        free(uri_bytes);
        free(token_bytes);
        free(name_bytes);
        return 0;
    }

    mpclipboard_Config *config = mpclipboard_config_new(uri_bytes, token_bytes, name_bytes);
    free(uri_bytes);
    free(token_bytes);
    free(name_bytes);

    return (jlong) (intptr_t) config;
}

JNIEXPORT jlong JNICALL
Java_dev_mpclipboard_android_Ffi_mpclipboard_1context_1new(
    JNIEnv *env,
    jclass clazz,
    jlong config_ptr
) {
    (void) env;
    (void) clazz;
    return (jlong) (intptr_t) mpclipboard_context_new((mpclipboard_Config *) (intptr_t) config_ptr);
}

JNIEXPORT jlong JNICALL
Java_dev_mpclipboard_android_Ffi_mpclipboard_1new(
    JNIEnv *env,
    jclass clazz,
    jlong context_ptr
) {
    (void) env;
    (void) clazz;
    return (jlong) (intptr_t) mpclipboard_new((mpclipboard_Context *) (intptr_t) context_ptr);
}

JNIEXPORT jint JNICALL
Java_dev_mpclipboard_android_Ffi_mpclipboard_1get_1fd(
    JNIEnv *env,
    jclass clazz,
    jlong client_ptr
) {
    (void) env;
    (void) clazz;
    return mpclipboard_get_fd((mpclipboard_MPClipboard *) (intptr_t) client_ptr);
}

JNIEXPORT jobject JNICALL
Java_dev_mpclipboard_android_Ffi_mpclipboard_1read(
    JNIEnv *env,
    jclass clazz,
    jlong client_ptr
) {
    (void) clazz;

    mpclipboard_Output output = mpclipboard_read((mpclipboard_MPClipboard *) (intptr_t) client_ptr);

    switch (output.tag) {
        case MPCLIPBOARD_OUTPUT_CONNECTIVITY_CHANGED:
            return new_output(
                env,
                (jint) output.tag,
                (jint) output.CONNECTIVITY_CHANGED.connectivity,
                NULL
            );
        case MPCLIPBOARD_OUTPUT_NEW_TEXT: {
            jsize len = (jsize) output.NEW_TEXT.len;
            jbyteArray text = (*env)->NewByteArray(env, len);
            if (text == NULL) {
                return NULL;
            }
            (*env)->SetByteArrayRegion(
                env,
                text,
                0,
                len,
                (const jbyte *) output.NEW_TEXT.ptr
            );
            if ((*env)->ExceptionCheck(env)) {
                return NULL;
            }

            return new_output(env, (jint) output.tag, 0, text);
        }
        case MPCLIPBOARD_OUTPUT_IGNORE:
            return NULL;
        case MPCLIPBOARD_OUTPUT_ERROR:
            throw_runtime_exception(env, "mpclipboard_read failed");
            return NULL;
        default:
            throw_runtime_exception(env, "mpclipboard_read returned unknown output tag");
            return NULL;
    }
}

JNIEXPORT jboolean JNICALL
Java_dev_mpclipboard_android_Ffi_mpclipboard_1push_1text2(
    JNIEnv *env,
    jclass clazz,
    jlong client_ptr,
    jbyteArray text
) {
    (void) clazz;

    if (text == NULL) {
        throw_runtime_exception(env, "text must not be null");
        return JNI_FALSE;
    }

    jsize len = (*env)->GetArrayLength(env, text);
    jbyte *bytes = (*env)->GetByteArrayElements(env, text, NULL);
    if (bytes == NULL) {
        return JNI_FALSE;
    }

    mpclipboard_PushResult result = mpclipboard_push_text2(
        (mpclipboard_MPClipboard *) (intptr_t) client_ptr,
        (const char *) bytes,
        (size_t) len
    );
    (*env)->ReleaseByteArrayElements(env, text, bytes, JNI_ABORT);

    switch (result) {
        case MPCLIPBOARD_PUSH_RESULT_SENT:
            return JNI_TRUE;
        case MPCLIPBOARD_PUSH_RESULT_DROPPED_AS_STALE:
            return JNI_FALSE;
        case MPCLIPBOARD_PUSH_RESULT_ERROR:
            throw_runtime_exception(env, "mpclipboard_push_text2 failed");
            return JNI_FALSE;
        default:
            throw_runtime_exception(env, "mpclipboard_push_text2 returned unknown result");
            return JNI_FALSE;
    }
}
