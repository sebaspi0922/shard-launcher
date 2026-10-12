#include <cstdio>
#include <unistd.h>

#include "my_application.h"

int main(int argc, char** argv) {
  int status;
  {
    g_autoptr(MyApplication) app = my_application_new();
    status = g_application_run(G_APPLICATION(app), argc, argv);
  }
  // Con el motor ya cerrado, los destructores de salida del EGL de NVIDIA
  // provocan un SIGSEGV. No queda nada por guardar: se sale sin ejecutarlos.
  fflush(stdout);
  fflush(stderr);
  _exit(status);
}
