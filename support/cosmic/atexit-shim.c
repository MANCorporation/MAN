/*
 * Buildroot's glibc exports __cxa_atexit but not the legacy atexit symbol
 * referenced by its shared libstdc++. Mesa's software DRI driver loads C++
 * at runtime, so provide the conventional wrapper for the COSMIC session.
 */
extern int __cxa_atexit(void (*function)(void *), void *argument, void *dso_handle);

int atexit(void (*function)(void))
{
    return __cxa_atexit((void (*)(void *))function, 0, 0);
}
