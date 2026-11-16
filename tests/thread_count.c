#define _GNU_SOURCE
#include <dlfcn.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>

static unsigned count;

int pthread_create(pthread_t *thread, const pthread_attr_t *attributes,
                   void *(*start)(void *), void *argument) {
    __sync_fetch_and_add(&count, 1);
    int (*next_create)(pthread_t *, const pthread_attr_t *, void *(*)(void *), void *) =
        dlsym(RTLD_NEXT, "pthread_create");
    return next_create(thread, attributes, start, argument);
}

__attribute__((destructor)) static void report_threads(void) {
    if (getenv("FZ_COUNT_THREADS")) fprintf(stderr, "%u\n", count);
}
