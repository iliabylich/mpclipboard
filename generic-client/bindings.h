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

typedef struct {
  bool error;
  bool has_connectivity;
  mpclipboard_Connectivity connectivity;
  mpclipboard_OwnedString text;
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
