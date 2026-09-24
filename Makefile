PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin
SYSTEMD_USER_DIR ?= $(HOME)/.config/systemd/user
CARGO ?= cargo
INSTALL ?= install
# The checked-in service uses the relocatable per-user default. When BINDIR is
# changed, rewrite its ExecStart to keep the service pointed at the installed binary.
SERVICE_BIN_DIR = $(if $(filter $(HOME)/.local/bin,$(BINDIR)),%h/.local/bin,$(BINDIR))
# Optional supported Linux Whisper GPU backend: empty (CPU), vulkan, cuda, or hip.
# Example: make release GPU=vulkan
GPU ?=
GPU_FLAG = $(if $(GPU),--features dictation-native/gpu-$(GPU),)

ifneq ($(strip $(GPU)),)
ifneq ($(words $(GPU)),1)
$(error GPU must be empty, vulkan, cuda, or hip; the full app currently targets Linux)
endif
ifeq ($(filter $(GPU),vulkan cuda hip),)
$(error GPU must be empty, vulkan, cuda, or hip; the full app currently targets Linux)
endif
endif

.PHONY: help build release check fmt install uninstall

help:
	@printf '%s\n' \
	  'make build      Build debug binaries' \
	  'make release    Build optimized release binaries (add GPU=vulkan|cuda|hip for a GPU build)' \
	  'make check      Type-check the workspace' \
	  'make fmt        Check Rust formatting' \
	  'make install    Build and install binaries, runtime libraries, and user service' \
	  'make uninstall  Remove installed binaries, runtime libraries, and user service files'

build:
	$(CARGO) build --workspace $(GPU_FLAG)

release:
	$(CARGO) build --workspace --release $(GPU_FLAG)

check:
	$(CARGO) check --workspace $(GPU_FLAG)

fmt:
	$(CARGO) fmt --all -- --check

install: release
	$(INSTALL) -Dm755 target/release/dictation "$(DESTDIR)$(BINDIR)/dictation"
	$(INSTALL) -Dm755 target/release/dictation-indicator "$(DESTDIR)$(BINDIR)/dictation-indicator"
	for lib in target/release/*.so; do \
	  test -e "$$lib" || continue; \
	  $(INSTALL) -Dm755 "$$lib" "$(DESTDIR)$(BINDIR)/$${lib##*/}"; \
	done
	$(INSTALL) -d "$(DESTDIR)$(SYSTEMD_USER_DIR)"
	sed 's|^ExecStart=.*|ExecStart=$(SERVICE_BIN_DIR)/dictation-indicator|' systemd/dictation-indicator.service > "$(DESTDIR)$(SYSTEMD_USER_DIR)/dictation-indicator.service"
	chmod 644 "$(DESTDIR)$(SYSTEMD_USER_DIR)/dictation-indicator.service"

uninstall:
	rm -f "$(DESTDIR)$(BINDIR)/dictation" \
	  "$(DESTDIR)$(BINDIR)/dictation-indicator" \
	  "$(DESTDIR)$(BINDIR)/libonnxruntime.so" \
	  "$(DESTDIR)$(BINDIR)/libsherpa-onnx-c-api.so" \
	  "$(DESTDIR)$(BINDIR)/libsherpa-onnx-cxx-api.so" \
	  "$(DESTDIR)$(SYSTEMD_USER_DIR)/dictation-indicator.service"
