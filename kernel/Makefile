KDIR ?= /lib/modules/$(shell uname -r)/build
PWD := $(shell pwd)

EXTRA_CFLAGS += -DCONFIG_ALIENWARE_WMI_WMAX -DCONFIG_ALIENWARE_WMI_LEGACY

obj-m += alienware-wmi.o
alienware-wmi-y := src/alienware-wmi-base.o src/alienware-wmi-wmax.o src/alienware-wmi-legacy.o

all:
	$(MAKE) -C $(KDIR) M=$(PWD) modules

clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean
