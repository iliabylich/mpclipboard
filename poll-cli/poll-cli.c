#include "../generic-client/bindings.h"
#include <assert.h>
#include <errno.h>
#include <poll.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

void print_output(mpclipboard_Output output);
void push_stdin_line(mpclipboard_MPClipboard *mpclipboard);

static char *RED = "\033[31m";
static char *GREEN = "\033[32m";
static char *YELLOW = "\033[33m";
static char *NC = "\033[0m";
static char *INFO =
    "\n\nThis is a demo of MPClipboard.\n"
    "It reads lines from stdin and sends them to MPClipboard.\n"
    "At the same time it polls MPClipboard and prints every received clip.\n\n";

int main() {
  printf("%s%s%s\n", GREEN, INFO, NC);

  assert(mpclipboard_init());

  mpclipboard_Config *config =
      mpclipboard_config_read(MPCLIPBOARD_CONFIG_READ_OPTION_FROM_LOCAL_FILE);
  assert(config != NULL);

  mpclipboard_Context *ctx = mpclipboard_context_new(config);
  assert(ctx != NULL);

  mpclipboard_MPClipboard *mpclipboard = mpclipboard_new(ctx);
  int mpclipboard_fd = mpclipboard_get_fd(mpclipboard);

  while (true) {
    // stdin sends new text into mpclipboard
    // mpclipboard fd emits incoming events
    struct pollfd fds[2] = {
        {.fd = STDIN_FILENO, .events = POLLIN, .revents = 0},
        {.fd = mpclipboard_fd, .events = POLLIN, .revents = 0},
    };

    int n = poll(fds, 2, -1);
    if (n == -1) {
      if (errno == EINTR) {
        continue;
      }
      perror("poll() failed");
      return 1;
    }

    if (fds[0].revents & POLLIN) {
      push_stdin_line(mpclipboard);
    }

    if (fds[1].revents & POLLIN) {
      mpclipboard_Output output = mpclipboard_read(mpclipboard);
      print_output(output);
    }
  }
}

void push_stdin_line(mpclipboard_MPClipboard *mpclipboard) {
  char buffer[4096];
  if (!fgets(buffer, sizeof(buffer), stdin)) {
    exit(0);
  }

  size_t len = strlen(buffer);
  if (len > 0 && buffer[len - 1] == '\n') {
    len--;
  }

  mpclipboard_push_text2(mpclipboard, buffer, len);
}

void print_connectivity(mpclipboard_Connectivity connectivity) {
  switch (connectivity) {
  case MPCLIPBOARD_CONNECTIVITY_CONNECTING: {
    printf("%sconnecting%s\n", RED, NC);
    break;
  }
  case MPCLIPBOARD_CONNECTIVITY_CONNECTED: {
    printf("%sconnected%s\n", RED, NC);
    break;
  }
  case MPCLIPBOARD_CONNECTIVITY_DISCONNECTED: {
    printf("%sdisconnected%s\n", RED, NC);
    break;
  }
  }
}

void print_output(mpclipboard_Output output) {
  switch (output.tag) {
  case MPCLIPBOARD_OUTPUT_CONNECTIVITY_CHANGED: {
    print_connectivity(output.CONNECTIVITY_CHANGED.connectivity);
    break;
  }
  case MPCLIPBOARD_OUTPUT_NEW_TEXT: {
    printf("%s%.*s%s\n", YELLOW, (int)output.NEW_TEXT.len, output.NEW_TEXT.ptr,
           NC);
    break;
  }
  case MPCLIPBOARD_OUTPUT_INTERNAL: {
    break;
  }
  }
}
