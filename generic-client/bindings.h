#pragma once

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

typedef enum {
  MPCLIPBOARD_CONNECTIVITY_CONNECTING,
  MPCLIPBOARD_CONNECTIVITY_CONNECTED,
  MPCLIPBOARD_CONNECTIVITY_DISCONNECTED,
} mpclipboard_Connectivity;

typedef enum {
  MPCLIPBOARD_PUSH_RESULT_PUSHED,
  MPCLIPBOARD_PUSH_RESULT_DROPPED,
  MPCLIPBOARD_PUSH_RESULT_ERROR,
} mpclipboard_PushResult;

typedef struct mpclipboard_MPClipboard mpclipboard_MPClipboard;

typedef struct {
  const char *ptr;
  size_t len;
} mpclipboard_BorrowedString;

typedef struct {
  char *ptr;
  size_t len;
} mpclipboard_OwnedString;

typedef enum {
  MPCLIPBOARD_OUTPUT_CONNECTIVITY_CHANGED,
  MPCLIPBOARD_OUTPUT_NEW_TEXT,
  MPCLIPBOARD_OUTPUT_BOTH,
  MPCLIPBOARD_OUTPUT_IGNORE,
  MPCLIPBOARD_OUTPUT_ERROR,
} mpclipboard_Output_Tag;

typedef struct {
  mpclipboard_Connectivity connectivity;
} mpclipboard_ConnectivityChanged_Body;

typedef struct {
  mpclipboard_OwnedString text;
} mpclipboard_NewText_Body;

typedef struct {
  mpclipboard_Connectivity connectivity;
  mpclipboard_OwnedString text;
} mpclipboard_Both_Body;

typedef struct {
  mpclipboard_Output_Tag tag;
  union {
    mpclipboard_ConnectivityChanged_Body CONNECTIVITY_CHANGED;
    mpclipboard_NewText_Body NEW_TEXT;
    mpclipboard_Both_Body BOTH;
  };
} mpclipboard_Output;

mpclipboard_MPClipboard *mpclipboard_new_inline(mpclipboard_BorrowedString url,
                                                mpclipboard_BorrowedString token,
                                                mpclipboard_BorrowedString id);

mpclipboard_MPClipboard *mpclipboard_new_with_local_config(void);

mpclipboard_MPClipboard *mpclipboard_new_with_xdg_config(void);

int32_t mpclipboard_get_fd(const mpclipboard_MPClipboard *mpclipboard);

mpclipboard_Output mpclipboard_read(mpclipboard_MPClipboard *mpclipboard);

mpclipboard_PushResult mpclipboard_push_text(mpclipboard_MPClipboard *mpclipboard,
                                             mpclipboard_BorrowedString text);

void mpclipboard_drop(mpclipboard_MPClipboard *mpclipboard);

void mpclipboard_drop_str(mpclipboard_OwnedString text);
