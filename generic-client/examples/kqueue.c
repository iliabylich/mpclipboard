#include "../bindings.h"
#include <errno.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/event.h>
#include <sys/time.h>
#include <unistd.h>

enum {
  SOURCE_TIMER = 1,
  SOURCE_MPCLIPBOARD = 2,
};

void print_output(mpclipboard_Output output);

int main() {
  mpclipboard_init();
  mpclipboard_logger_test();

  mpclipboard_Config *config =
      mpclipboard_config_read(MPCLIPBOARD_CONFIG_READ_OPTION_FROM_LOCAL_FILE);
  if (!config) {
    perror("NULL config");
    exit(1);
  }
  mpclipboard_Context *ctx = mpclipboard_context_new(config);
  if (!ctx) {
    perror("NULL context");
    exit(1);
  }

  mpclipboard_MPClipboard *mpclipboard = mpclipboard_new(ctx);
  int fd = mpclipboard_get_fd(mpclipboard);

  int kq = kqueue();
  if (kq == -1) {
    perror("kqueue() failed");
    return 1;
  }

  struct kevent ev;
  // timer
  EV_SET(&ev, 1, EVFILT_TIMER, EV_ADD | EV_ENABLE, 0, 1000,
         (void *)SOURCE_TIMER);
  if (kevent(kq, &ev, 1, NULL, 0, NULL) == -1) {
    perror("kevent(add timer) failed");
    exit(1);
  }
  // mpclipboard
  EV_SET(&ev, (uintptr_t)fd, EVFILT_READ, EV_ADD | EV_ENABLE, 0, 0,
         (void *)SOURCE_MPCLIPBOARD);

  if (kevent(kq, &ev, 1, NULL, 0, NULL) == -1) {
    perror("kevent(add mpclipboard) failed");
    exit(1);
  }

  while (true) {
    struct kevent events[8];

    int n = kevent(kq, NULL, 0, events, 8, NULL);
    if (n == -1) {
      if (errno == EINTR) {
        continue;
      }
      perror("kevent(wait) failed");
      close(kq);
      return 1;
    }

    for (int i = 0; i < n; i++) {
      struct kevent *ev = &events[i];
      int source = (int)(size_t)ev->udata;

      if (ev->flags & EV_ERROR) {
        fprintf(stderr,
                "event error: source=%s ident=%lu filter=%d data=%lld: %s\n",
                source == SOURCE_TIMER         ? "timer"
                : source == SOURCE_MPCLIPBOARD ? "mpclipboard"
                                               : "unknown",
                (unsigned long)ev->ident, ev->filter, (long long)ev->data,
                strerror((int)ev->data));
        continue;
      }

      switch (source) {
      case SOURCE_TIMER: {
        break;
      }

      case SOURCE_MPCLIPBOARD: {
        mpclipboard_Output output = mpclipboard_read(mpclipboard);

        print_output(output);

        break;
      }

      default:
        printf("[unknown] source=%d ident=%lu filter=%d flags=0x%x\n", source,
               (unsigned long)ev->ident, ev->filter, ev->flags);
        break;
      }
    }
  }
}

void print_connectivity(mpclipboard_Connectivity connectivity) {
  switch (connectivity) {
  case MPCLIPBOARD_CONNECTIVITY_CONNECTING: {
    printf("connecting\n");
    break;
  }
  case MPCLIPBOARD_CONNECTIVITY_CONNECTED: {
    printf("connected\n");
    break;
  }
  case MPCLIPBOARD_CONNECTIVITY_DISCONNECTED: {
    printf("disconnected\n");
    break;
  }
  }
}

void print_string(char *ptr, size_t len) { printf("%.*s\n", (int)len, ptr); }

void print_output(mpclipboard_Output output) {
  switch (output.tag) {
  case MPCLIPBOARD_OUTPUT_CONNECTIVITY_CHANGED: {
    print_connectivity(output.CONNECTIVITY_CHANGED.connectivity);
    break;
  }
  case MPCLIPBOARD_OUTPUT_NEW_TEXT: {
    print_string(output.NEW_TEXT.ptr, output.NEW_TEXT.len);
    break;
  }
  case MPCLIPBOARD_OUTPUT_NEW_BINARY: {
    print_string(output.NEW_BINARY.ptr, output.NEW_BINARY.len);
    break;
  }
  case MPCLIPBOARD_OUTPUT_NEW_TEXT_AND_CONNECTIVITY_CHANGED: {
    print_connectivity(output.NEW_TEXT_AND_CONNECTIVITY_CHANGED.connectivity);
    print_string(output.NEW_TEXT_AND_CONNECTIVITY_CHANGED.ptr,
                 output.NEW_TEXT_AND_CONNECTIVITY_CHANGED.len);
    break;
  }
  case MPCLIPBOARD_OUTPUT_NEW_BINARY_AND_CONNECTIVITY_CHANGED: {
    print_connectivity(output.NEW_BINARY_AND_CONNECTIVITY_CHANGED.connectivity);
    print_string(output.NEW_BINARY_AND_CONNECTIVITY_CHANGED.ptr,
                 output.NEW_BINARY_AND_CONNECTIVITY_CHANGED.len);
    break;
  }
  case MPCLIPBOARD_OUTPUT_INTERNAL: {
    break;
  }
  }
}
