CURRENT_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

TOCK_ARCH ?= cortex-m3
TOCK_USERLAND_BASE_DIR = $(CURRENT_DIR)../../../../libtock-c
BUILDDIR ?= $(CURRENT_DIR)/build/$(TOCK_ARCH)

C_SRCS   := $(wildcard *.c)

OBJS += $(patsubst %.c,$(BUILDDIR)/%.o,$(C_SRCS))

TOCK_APP_CONFIG = -Xlinker --defsym=STACK_SIZE=$$(STACK_SIZE)\
                  -Xlinker --defsym=APP_HEAP_SIZE=$$(APP_HEAP_SIZE)\
                  -Xlinker --defsym=KERNEL_HEAP_SIZE=$$(KERNEL_HEAP_SIZE)


include $(TOCK_USERLAND_BASE_DIR)/AppMakefile.mk

$(BUILDDIR)/%.o: %.c | $(BUILDDIR)
	$(CC) $(CFLAGS) $(CPPFLAGS) -c -o $@ $<

