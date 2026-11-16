#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <stdlib.h>
#include <sys/stat.h>
#include <unistd.h>

ssize_t read(int fd, void *buffer, size_t length) {
    static int injected;
    struct stat source, input;
    if (!injected && getenv("FZ_READ_ERROR") && fstat(fd, &source) == 0 &&
        fstat(STDIN_FILENO, &input) == 0 && source.st_dev == input.st_dev &&
        source.st_ino == input.st_ino) {
        injected = 1;
        errno = atoi(getenv("FZ_READ_ERROR"));
        return -1;
    }
    ssize_t (*next_read)(int, void *, size_t) = dlsym(RTLD_NEXT, "read");
    return next_read(fd, buffer, length);
}
