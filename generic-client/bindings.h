#pragma once

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

/**
 * Instruction for the `Config::read` function how to read the config.
 */
typedef enum {
  /**
   * Read from "./config.toml", based on your current working directory
   */
  MPCLIPBOARD_CONFIG_READ_OPTION_FROM_LOCAL_FILE = 0,
  /**
   * Read from XDG Config dir (i.e. from `~/.config/mpclipboard/config.toml`)
   */
  MPCLIPBOARD_CONFIG_READ_OPTION_FROM_XDG_CONFIG_DIR = 1,
} mpclipboard_ConfigReadOption;

/**
 * Connectivity of the MPClipboard, emitted in `on_connectivity_changed`
 */
typedef enum {
  /**
   * Connecting to remote server, performing handshake/auth
   */
  MPCLIPBOARD_CONNECTIVITY_CONNECTING,
  /**
   * Connected, ready to talk
   */
  MPCLIPBOARD_CONNECTIVITY_CONNECTED,
  /**
   * Disconnected
   */
  MPCLIPBOARD_CONNECTIVITY_DISCONNECTED,
} mpclipboard_Connectivity;

/**
 * Representation of a runtime configuration
 */
typedef struct mpclipboard_Config mpclipboard_Config;

/**
 * Execution context of MPClipboard, once constructed nothing can fail
 */
typedef struct mpclipboard_Context mpclipboard_Context;

/**
 * The main entrypoint
 */
typedef struct mpclipboard_MPClipboard mpclipboard_MPClipboard;

/**
 * Result of reading
 */
typedef enum {
  /**
   * An event indicating that connectivity changed, guaranteed to be different from a previous one
   */
  MPCLIPBOARD_OUTPUT_CONNECTIVITY_CHANGED,
  /**
   * New text clip
   */
  MPCLIPBOARD_OUTPUT_NEW_TEXT,
  /**
   * New binary clip
   */
  MPCLIPBOARD_OUTPUT_NEW_BINARY,
  /**
   * Internal
   */
  MPCLIPBOARD_OUTPUT_INTERNAL,
} mpclipboard_Output_Tag;

typedef struct {
  /**
   * New connecivity
   */
  mpclipboard_Connectivity connectivity;
} mpclipboard_ConnectivityChanged_Body;

typedef struct {
  /**
   * New text
   */
  char *ptr;
  /**
   * and its length
   */
  size_t len;
} mpclipboard_NewText_Body;

typedef struct {
  /**
   * New bytes
   */
  char *ptr;
  /**
   * and its length
   */
  size_t len;
} mpclipboard_NewBinary_Body;

typedef struct {
  mpclipboard_Output_Tag tag;
  union {
    mpclipboard_ConnectivityChanged_Body CONNECTIVITY_CHANGED;
    mpclipboard_NewText_Body NEW_TEXT;
    mpclipboard_NewBinary_Body NEW_BINARY;
  };
} mpclipboard_Output;

/**
 * Initializes MPClipboard, must be called once at startup
 */
bool mpclipboard_init(void);

/**
 * Reads the config based on the given instruction
 * (which is either "read from XDG dir" or "read from local ./config.toml").
 * In case of an error logs it and returns NULL.
 */
mpclipboard_Config *mpclipboard_config_read(mpclipboard_ConfigReadOption option);

/**
 * Constructs the config in-place based on given parameters that match fields 1-to-1.
 * In case of an error logs it and returns NULL.
 */
mpclipboard_Config *mpclipboard_config_new(const char *uri, const char *token, const char *name);

/**
 * Constructs a new MPClipboard context.
 * Consumes config.
 */
mpclipboard_Context *mpclipboard_context_new(mpclipboard_Config *config);

/**
 * Constructs a new MPClipboard.
 * Consumes context.
 */
mpclipboard_MPClipboard *mpclipboard_new(mpclipboard_Context *context);

/**
 * Constructs a new MPClipboard.
 * Consumes context.
 */
int32_t mpclipboard_get_fd(mpclipboard_MPClipboard *mpclipboard);

/**
 * Reads from a given MPClipboard instance.
 */
mpclipboard_Output mpclipboard_read(mpclipboard_MPClipboard *mpclipboard);

/**
 * Pushes text from NULL-terminated C-style string
 */
bool mpclipboard_push_text1(mpclipboard_MPClipboard *mpclipboard, const char *text);

/**
 * Pushes text from pointer + length
 */
bool mpclipboard_push_text2(mpclipboard_MPClipboard *mpclipboard, const char *ptr, size_t len);

/**
 * Pushes binary
 */
bool mpclipboard_push_binary(mpclipboard_MPClipboard *mpclipboard, const char *ptr, size_t len);

/**
 * Drops an instance of MPClipboard, frees memory, closes files
 */
void mpclipboard_drop(mpclipboard_MPClipboard *mpclipboard);

/**
 * Prints one "info" and one "error" message, useful for testing
 */
void mpclipboard_logger_test(void);
