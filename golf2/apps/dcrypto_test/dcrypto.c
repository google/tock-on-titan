#include <tock.h>
#include "dcrypto.h"


int last_error = 0;
int last_fault = 0;

static void tock_dcrypto_run_done(int error,
			      int fault,
			      int unused __attribute__((unused)),
			      void *callback_args) {
  last_error = error;
  last_fault = fault;
  *(bool*)callback_args = true;
}



int tock_dcrypto_run(void* data, size_t datalen,
		     void* program, size_t programlen) {

  int ret = -1;
  bool run_done = false;
    
  ret = subscribe(HOTEL_DRIVER_DCRYPTO, TOCK_DCRYPTO_RUN_DONE,
		  tock_dcrypto_run_done, &run_done);
  if (ret < 0) {
    printf("Could not register dcrypto callback with kernel: %d\n", ret);
    return ret;
  }

  ret = allow(HOTEL_DRIVER_DCRYPTO, TOCK_DCRYPTO_ALLOW_DATA,
	      data, datalen);
  if (ret < 0) {

    printf("Could not give kernel access to dcrypto data: %d\n", ret);
    return ret;
  }

  ret = allow(HOTEL_DRIVER_DCRYPTO, TOCK_DCRYPTO_ALLOW_PROG,
	      program, programlen);
  if (ret < 0) {
    printf("Could not give kernel access to dcrypto program: %d\n", ret);
    return ret;
  }

  ret = command(HOTEL_DRIVER_DCRYPTO, TOCK_DCRYPTO_CMD_RUN, 0, 0);

  if (ret < 0) {
    printf("Could not invoke dcrypto program with command: %d\n", ret);
    return ret;
  }

  yield_for(&run_done);

  if (last_error != 0) {
    printf("DCRYPTO failed with fault %i.\n", last_fault);
    return last_fault;
  } else {
    return 0;
  }
}

