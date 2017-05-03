export PATH := /home/esafronov/.cargo/bin:$(PATH)

PREFIX = /usr

.PHONY: all clean install

all: lib/liberty

lib/liberty:
		cargo build --release
install: lib/liberty
		mkdir -p $(DESTDIR)$(PREFIX)/lib
		mkdir -p $(DESTDIR)$(PREFIX)/include/liberty
		cp target/release/liberty.so $(DESTDIR)$(PREFIX)/lib/liberty.so.0
		cp include/liberty/liberty.h $(DESTDIR)$(PREFIX)/include/liberty
		cp include/liberty/liberty.hpp $(DESTDIR)$(PREFIX)/include/liberty
