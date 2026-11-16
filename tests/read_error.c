#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <linux/filter.h>
#include <linux/seccomp.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/stat.h>
#include <sys/signalfd.h>
#include <sys/prctl.h>
#include <sys/syscall.h>
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
    if (!injected && getenv("FZ_TTY_READ_ERROR") && isatty(fd)) {
        injected = 1;
        errno = atoi(getenv("FZ_TTY_READ_ERROR"));
        return -1;
    }
    ssize_t (*next_read)(int, void *, size_t) = dlsym(RTLD_NEXT, "read");
    return next_read(fd, buffer, length);
}

int signalfd(int fd, const sigset_t *mask, int flags) {
    if (getenv("FZ_SIGNALFD_ERROR") && fd == -1) {
        errno = atoi(getenv("FZ_SIGNALFD_ERROR"));
        return -1;
    }
    int (*next_signalfd)(int, const sigset_t *, int) = dlsym(RTLD_NEXT, "signalfd");
    return next_signalfd(fd, mask, flags);
}

__attribute__((constructor)) static void refuse_pending_poll(void) {
    if (!getenv("FZ_PENDING_POLL_ERROR")) return;
    // Rustix issues raw syscalls on Linux. Refuse timed ppoll in this disposable
    // process; these probes contain no Escape, so its only timeout is the readiness poll.
    struct sock_filter filter[] = {
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, nr)),
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, __NR_ppoll, 0, 5),
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, args[2])),
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, 0, 0, 2),
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, args[2]) + 4),
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, 0, 1, 0),
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ERRNO | ENOMEM),
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ALLOW),
    };
    struct sock_fprog program = { .len = sizeof(filter) / sizeof(filter[0]), .filter = filter };
    if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) == -1 ||
        prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &program) == -1) {
        fputs("FZ_HARNESS_ERROR: could not install poll refusal\n", stderr);
        _exit(86);
    }
}
