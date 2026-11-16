#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <stdlib.h>
#include <unistd.h>

ssize_t read(int fd, void *buffer, size_t length) {
    static int injected;
    if (fd == STDIN_FILENO && !injected && getenv("FZ_READ_ERROR")) {
        injected = 1;
        errno = atoi(getenv("FZ_READ_ERROR"));
        return -1;
    }
    ssize_t (*next_read)(int, void *, size_t) = dlsym(RTLD_NEXT, "read");
    return next_read(fd, buffer, length);
}
