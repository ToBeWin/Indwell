.PHONY: verify-nonhardware smoke-host-sim

verify-nonhardware:
	bash scripts/verify-nonhardware.sh

smoke-host-sim:
	bash scripts/smoke-host-sim.sh
