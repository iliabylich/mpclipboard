#include <jni.h>
#include <limits.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "bindings.h"

#define FATAL(MESSAGE)                                                         \
  do {                                                                         \
    (*env)->FatalError(env, MESSAGE);                                          \
    abort();                                                                   \
  } while (0)

#define CHECK(CONDITION, MESSAGE)                                              \
  do {                                                                         \
    if (!(CONDITION))                                                          \
      FATAL(MESSAGE);                                                          \
  } while (0)

static char *jstring_to_c_string(JNIEnv *env, jstring string) {
  CHECK(string != NULL, "string argument must not be null");

  jclass string_class = (*env)->FindClass(env, "java/lang/String");
  CHECK(string_class != NULL, "failed to find java.lang.String");
  jmethodID get_bytes = (*env)->GetMethodID(env, string_class, "getBytes",
                                            "(Ljava/lang/String;)[B");
  CHECK(get_bytes != NULL, "failed to find String.getBytes(String)");
  jstring utf8 = (*env)->NewStringUTF(env, "UTF-8");
  CHECK(utf8 != NULL, "failed to create UTF-8 charset name");
  jbyteArray bytes =
      (jbyteArray)(*env)->CallObjectMethod(env, string, get_bytes, utf8);
  CHECK(bytes != NULL, "failed to encode string as UTF-8");

  jsize len = (*env)->GetArrayLength(env, bytes);
  char *buffer = malloc((size_t)len + 1U);
  CHECK(buffer != NULL, "failed to allocate string buffer");
  (*env)->GetByteArrayRegion(env, bytes, 0, len, (jbyte *)buffer);
  CHECK(!(*env)->ExceptionCheck(env), "failed to copy UTF-8 bytes");
  CHECK(memchr(buffer, '\0', (size_t)len) == NULL,
        "string argument contains a NUL byte");
  buffer[len] = '\0';
  return buffer;
}

static jobject box_int(JNIEnv *env, jint value) {
  jclass integer_class = (*env)->FindClass(env, "java/lang/Integer");
  CHECK(integer_class != NULL, "failed to find java.lang.Integer");
  jmethodID value_of = (*env)->GetStaticMethodID(env, integer_class, "valueOf",
                                                 "(I)Ljava/lang/Integer;");
  CHECK(value_of != NULL, "failed to find Integer.valueOf(int)");
  jobject boxed =
      (*env)->CallStaticObjectMethod(env, integer_class, value_of, value);
  CHECK(boxed != NULL, "failed to box integer");
  return boxed;
}

static jobject new_pair(JNIEnv *env, jobject first, jobject second) {
  jclass pair_class = (*env)->FindClass(env, "kotlin/Pair");
  CHECK(pair_class != NULL, "failed to find kotlin.Pair");
  jmethodID ctor = (*env)->GetMethodID(
      env, pair_class, "<init>", "(Ljava/lang/Object;Ljava/lang/Object;)V");
  CHECK(ctor != NULL, "failed to find Pair constructor");
  jobject pair = (*env)->NewObject(env, pair_class, ctor, first, second);
  CHECK(pair != NULL, "failed to construct Pair");
  return pair;
}

static jstring new_jstring(JNIEnv *env, char *ptr, size_t len) {
  CHECK(len <= INT_MAX, "clipboard text exceeds Java array limit");

  jbyteArray bytes = (*env)->NewByteArray(env, (jsize)len);
  CHECK(bytes != NULL, "failed to allocate Java byte array");
  (*env)->SetByteArrayRegion(env, bytes, 0, (jsize)len, (const jbyte *)ptr);
  CHECK(!(*env)->ExceptionCheck(env),
        "failed to copy native text into Java array");
  mpclipboard_drop_str(ptr, len);

  jclass string_class = (*env)->FindClass(env, "java/lang/String");
  CHECK(string_class != NULL, "failed to find java.lang.String");
  jmethodID ctor = (*env)->GetMethodID(env, string_class, "<init>",
                                       "([BLjava/lang/String;)V");
  CHECK(ctor != NULL, "failed to find String(byte[], String) constructor");
  jstring utf8 = (*env)->NewStringUTF(env, "UTF-8");
  CHECK(utf8 != NULL, "failed to create UTF-8 charset name");
  jstring string =
      (jstring)(*env)->NewObject(env, string_class, ctor, bytes, utf8);
  CHECK(string != NULL, "failed to construct Java string");
  return string;
}

JNIEXPORT void JNICALL Java_dev_ibylich_mpclipboard_Ffi_mpclipboard_1fatal(
    JNIEnv *env, [[maybe_unused]] jclass clazz, jstring message) {
  char *message_bytes = jstring_to_c_string(env, message);
  FATAL(message_bytes);
}

JNIEXPORT void JNICALL
Java_dev_ibylich_mpclipboard_Ffi_mpclipboard_1define_1enums(
    JNIEnv *env, [[maybe_unused]] jclass clazz) {
  jclass connectivity =
      (*env)->FindClass(env, "dev/ibylich/mpclipboard/Ffi$Connectivity");
  CHECK(connectivity != NULL, "failed to find Ffi.Connectivity");
  jclass push_result =
      (*env)->FindClass(env, "dev/ibylich/mpclipboard/Ffi$PushResult");
  CHECK(push_result != NULL, "failed to find Ffi.PushResult");

#define SET(CLASS, NAME, VALUE)                                                \
  do {                                                                         \
    jfieldID field = (*env)->GetStaticFieldID(env, CLASS, #NAME, "I");         \
    CHECK(field != NULL, "failed to find field " #NAME);                       \
    (*env)->SetStaticIntField(env, CLASS, field, VALUE);                       \
    CHECK(!(*env)->ExceptionCheck(env), "failed to set field " #NAME);         \
  } while (0)

  SET(connectivity, CONNECTING, MPCLIPBOARD_CONNECTIVITY_CONNECTING);
  SET(connectivity, CONNECTED, MPCLIPBOARD_CONNECTIVITY_CONNECTED);
  SET(connectivity, DISCONNECTED, MPCLIPBOARD_CONNECTIVITY_DISCONNECTED);

  SET(push_result, PUSHED, MPCLIPBOARD_PUSH_RESULT_PUSHED);
  SET(push_result, DROPPED, MPCLIPBOARD_PUSH_RESULT_DROPPED);

#undef SET
}

JNIEXPORT jlong JNICALL
Java_dev_ibylich_mpclipboard_Ffi_mpclipboard_1new_1inline(
    JNIEnv *env, [[maybe_unused]] jclass clazz, jstring uri, jstring token,
    jstring name) {

  char *uri_bytes = jstring_to_c_string(env, uri);
  char *token_bytes = jstring_to_c_string(env, token);
  char *name_bytes = jstring_to_c_string(env, name);

  mpclipboard_MPClipboard *mpclipboard =
      mpclipboard_new_inline(uri_bytes, token_bytes, name_bytes);
  free(uri_bytes);
  free(token_bytes);
  free(name_bytes);

  return (jlong)(intptr_t)mpclipboard;
}

JNIEXPORT jint JNICALL Java_dev_ibylich_mpclipboard_Ffi_mpclipboard_1get_1fd(
    JNIEnv *env, [[maybe_unused]] jclass clazz, jlong mpclipboard_ptr) {
  mpclipboard_MPClipboard *mpclipboard =
      (mpclipboard_MPClipboard *)(intptr_t)mpclipboard_ptr;
  CHECK(mpclipboard != NULL, "mpclipboard pointer must not be null");
  return mpclipboard_get_fd(mpclipboard);
}

JNIEXPORT void JNICALL Java_dev_ibylich_mpclipboard_Ffi_mpclipboard_1drop(
    [[maybe_unused]] JNIEnv *env, [[maybe_unused]] jclass clazz,
    jlong mpclipboard_ptr) {

  mpclipboard_MPClipboard *mpclipboard =
      (mpclipboard_MPClipboard *)(intptr_t)mpclipboard_ptr;
  CHECK(mpclipboard != NULL, "mpclipboard pointer must not be null");
  mpclipboard_drop(mpclipboard);
}

JNIEXPORT jobject JNICALL Java_dev_ibylich_mpclipboard_Ffi_mpclipboard_1read(
    JNIEnv *env, [[maybe_unused]] jclass clazz, jlong mpclipboard_ptr) {
  mpclipboard_MPClipboard *mpclipboard =
      (mpclipboard_MPClipboard *)(intptr_t)mpclipboard_ptr;
  CHECK(mpclipboard != NULL, "mpclipboard pointer must not be null");
  mpclipboard_Output output = mpclipboard_read(mpclipboard);
  jobject connectivity = NULL;
  jstring text = NULL;

  switch (output.tag) {
  case MPCLIPBOARD_OUTPUT_CONNECTIVITY_CHANGED:
    connectivity = box_int(env, (jint)output.CONNECTIVITY_CHANGED.connectivity);
    break;
  case MPCLIPBOARD_OUTPUT_NEW_TEXT:
    text = new_jstring(env, output.NEW_TEXT.ptr, output.NEW_TEXT.len);
    break;
  case MPCLIPBOARD_OUTPUT_BOTH:
    connectivity = box_int(env, (jint)output.BOTH.connectivity);
    text = new_jstring(env, output.BOTH.ptr, output.BOTH.len);
    break;
  case MPCLIPBOARD_OUTPUT_IGNORE:
    return NULL;
  case MPCLIPBOARD_OUTPUT_ERROR:
    FATAL("mpclipboard_read failed");
  default:
    FATAL("mpclipboard_read returned unknown output tag");
  }

  return new_pair(env, connectivity, text);
}

JNIEXPORT jint JNICALL Java_dev_ibylich_mpclipboard_Ffi_mpclipboard_1push_1text(
    JNIEnv *env, [[maybe_unused]] jclass clazz, jlong mpclipboard_ptr,
    jstring text) {

  mpclipboard_MPClipboard *mpclipboard =
      (mpclipboard_MPClipboard *)(intptr_t)mpclipboard_ptr;
  CHECK(mpclipboard != NULL, "mpclipboard pointer must not be null");
  char *bytes = jstring_to_c_string(env, text);
  size_t len = strlen(bytes);

  mpclipboard_PushResult push_result =
      mpclipboard_push_text(mpclipboard, bytes, len);
  free(bytes);

  switch (push_result) {
  case MPCLIPBOARD_PUSH_RESULT_PUSHED:
  case MPCLIPBOARD_PUSH_RESULT_DROPPED:
    return (jint)push_result;
  case MPCLIPBOARD_PUSH_RESULT_ERROR:
    FATAL("mpclipboard_push_text failed");
  default:
    FATAL("mpclipboard_push_text returned unknown push result");
  }
}
