# msectrax

Millisecond Insect Tracker - Source code

**Main directories**
- `msectrax-firmware` - source code for the firmware
- `msectrax-proxy` - source code for the browser user interface
- `py-msectrax` - source code for calibration and analysis

**Bundled dependencies**

- `dac714`
- `dac714-linux-test`
- `mini-rxtx`
- `msectrax-comms`
- `yew-tincture`

**Printed circuit boards**

- `Galvo_control_board` is the main board for mounting the NucleoF103RB board
  with two integrated DAC714 chips for output analog signals to drive the
  galvo motors.

- `QPD_amplifier` is PCB of the QPD with transimpedance amplifiers, differential
  amplifiers, and conditioners to convert the QPD signal to analog input of the
  Galvo_control_board.

## license

GPLv1

Other license conditions may be possible. Contract Andrew Straw.
