# Makefile for chromiumos-ec.a library. This includes code under third
# party licenses.

CHROMIUM_CFILES =  $(wildcard $(CHROMIUMOS_DIR)/*.c)
CHROMIUM_OBJS   =  $(notdir $(patsubst %.c, %.o, $(CHROMIUM_CFILES)))

define CHROMIUM_EC_RULES

$$(BUILDDIR)/$(1)/chromiumos-ec:
	$$(TRACEDIR)
	$$(Q)mkdir -p $$@

$(BUILDDIR)/$(1)/chromiumos-ec/sha256.o: $(CHROMIUMOS_DIR)/sha256.c | $$(BUILDDIR)/$(1)/chromiumos-ec
#	echo $(BUILDDIR)
	$$(Q)$$(CC) $$(CFLAGS) -mcpu=$(1) $$(CPPFLAGS) $$(CPPFLAGS_$(1)) -c -o $$@ $$<

CHROMIUM_$(1)_OBJS = $$(patsubst %.o, $$(BUILDDIR)/$(1)/chromiumos-ec/%.o, $$(CHROMIUM_OBJS))

$$(BUILDDIR)/$(1)/chromiumos-ec.a: $$(CHROMIUM_$(1)_OBJS)
#	echo $(CHROMIUM_$(1)_OBJS)
	ar rcs $$@ $$^

endef

$(foreach arch, $(TOCK_ARCHS), $(eval $(call CHROMIUM_EC_RULES,$(arch))))

#$(info $(foreach arch,$(TOCK_ARCHS),$(call CHROMIUM_EC_RULES,$(arch))))
