# Makefile for chromiumos-ec.a library. This includes code under third
# party licenses.

LIBGOLF2_CFILES =  $(wildcard $(LIBGOLF2_DIR)/*.c)
LIBGOLF2_OBJS   =  $(notdir $(patsubst %.c, %.o, $(LIBGOLF2_CFILES)))

$(C_SRCS):	$(LIBGOLF2_DIR)/

define LIBGOLF2_RULES

$$(BUILDDIR)/$(1)/libgolf2:
	$$(TRACEDIR)
	$$(Q)mkdir -p $$@

$(BUILDDIR)/$(1)/chromiumos-ec/%.o: $(CHROMIUMOS_DIR)/%.c | $$(BUILDDIR)/$(1)/libgolf2
	$$(Q)$$(CC) $$(CFLAGS) -mcpu=$(1) $$(CPPFLAGS) $$(CPPFLAGS_$(1)) -c -o $$@ $$<

LIBGOLF2_$(1)_OBJS = $$(patsubst %.o, $$(BUILDDIR)/$(1)/libgolf2/%.o, $$(LIBGOLF2_OBJS))

$$(BUILDDIR)/$(1)/libgolf2.a: $$(LIBGOLF2_$(1)_OBJS)
	ar rcs $$@ $$^

endef

$(foreach arch, $(TOCK_ARCHS), $(eval $(call LIBGOLF2_RULES,$(arch))))

