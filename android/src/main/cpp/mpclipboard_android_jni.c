#include <jni.h>
#include <limits.h>
#include <stdint.h>
#include <stdlib.h>

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

typedef struct {
  jbyteArray array;
  mpclipboard_BorrowedString str;
} bytes_t;

static bytes_t bytes_acquire(JNIEnv *env, jbyteArray array) {
  CHECK(array != NULL, "byte array argument must not be null");
  jbyte *ptr = (*env)->GetByteArrayElements(env, array, NULL);
  CHECK(ptr != NULL, "failed to access byte array");
  jsize len = (*env)->GetArrayLength(env, array);
  return (bytes_t){
      .array = array,
      .str = {.ptr = (const char *)ptr, .len = (size_t)len},
  };
}

static void bytes_release(JNIEnv *env, bytes_t bytes) {
  (*env)->ReleaseByteArrayElements(env, bytes.array, (jbyte *)bytes.str.ptr,
                                   JNI_ABORT);
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

static jbyteArray new_jbytearray(JNIEnv *env, mpclipboard_OwnedString text) {
  CHECK(text.len <= INT_MAX, "clipboard text exceeds Java array limit");

  jbyteArray bytes = (*env)->NewByteArray(env, (jsize)text.len);
  CHECK(bytes != NULL, "failed to allocate Java byte array");
  (*env)->SetByteArrayRegion(env, bytes, 0, (jsize)text.len,
                             (const jbyte *)text.ptr);
  CHECK(!(*env)->ExceptionCheck(env),
        "failed to copy native text into Java array");
  mpclipboard_drop_str(text);
  return bytes;
}

#define CONSTANT(NAME, VALUE)                                                  \
  JNIEXPORT jint JNICALL Java_dev_ibylich_mpclipboard_Ffi_##NAME(              \
      [[maybe_unused]] JNIEnv *env, [[maybe_unused]] jclass clazz) {           \
    return VALUE;                                                              \
  }

CONSTANT(connectivityConnecting, MPCLIPBOARD_CONNECTIVITY_CONNECTING)
CONSTANT(connectivityConnected, MPCLIPBOARD_CONNECTIVITY_CONNECTED)
CONSTANT(connectivityDisconnected, MPCLIPBOARD_CONNECTIVITY_DISCONNECTED)

#undef CONSTANT

JNIEXPORT
jlong JNICALL Java_dev_ibylich_mpclipboard_Ffi_mpclipboard_1new_1inline(
    JNIEnv *env, [[maybe_unused]] jclass clazz, jbyteArray uri,
    jbyteArray token, jbyteArray name) {

  bytes_t uri_bytes = bytes_acquire(env, uri);
  bytes_t token_bytes = bytes_acquire(env, token);
  bytes_t name_bytes = bytes_acquire(env, name);

  mpclipboard_MPClipboard *mpclipboard =
      mpclipboard_new_inline(uri_bytes.str, token_bytes.str, name_bytes.str);
  bytes_release(env, uri_bytes);
  bytes_release(env, token_bytes);
  bytes_release(env, name_bytes);

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
  CHECK(!output.error, "mpclipboard_read failed");

  jobject connectivity = NULL;
  jbyteArray text = NULL;

  if (output.has_connectivity) {
    connectivity = box_int(env, (jint)output.connectivity);
  }
  if (output.text.ptr != NULL) {
    text = new_jbytearray(env, output.text);
  }

  if (connectivity == NULL && text == NULL) {
    return NULL;
  }
  return new_pair(env, connectivity, text);
}

JNIEXPORT void JNICALL Java_dev_ibylich_mpclipboard_Ffi_mpclipboard_1push_1text(
    JNIEnv *env, [[maybe_unused]] jclass clazz, jlong mpclipboard_ptr,
    jbyteArray text) {

  mpclipboard_MPClipboard *mpclipboard =
      (mpclipboard_MPClipboard *)(intptr_t)mpclipboard_ptr;
  CHECK(mpclipboard != NULL, "mpclipboard pointer must not be null");
  bytes_t text_bytes = bytes_acquire(env, text);

  mpclipboard_PushResult push_result =
      mpclipboard_push_text(mpclipboard, text_bytes.str);
  bytes_release(env, text_bytes);

  switch (push_result) {
  case MPCLIPBOARD_PUSH_RESULT_PUSHED:
  case MPCLIPBOARD_PUSH_RESULT_DROPPED:
    return;
  case MPCLIPBOARD_PUSH_RESULT_ERROR:
    FATAL("mpclipboard_push_text failed");
  default:
    FATAL("mpclipboard_push_text returned unknown push result");
  }
}
