# RDUM
Rust-based drum machine

## Motivation
Part of my learning process for any language must include making musical things. I'm not a drummer, and I already spend too much on instruments to buy another one. Luckily my computer keeps pretty good time. Except now I'm making this instead of jamming! Some interesting challenges that arise when making any digital instrument:
- There is a very limited amount of time to process before we must play the next audio frame otherwise we get stuttering and other unpleasant artifacts
- Timing has to be precise because too much latency or drift can really ruin playing along with other instruments
- Interacting with the instrument should ideally be pleasant and intuitive (at least for me, no doubt the primary user of this thing)
I'm just playing around with Rust so we'll see how far we get, but one day this can perhaps run on a Raspberry Pi and be completely independent, with MIDI clock and line outs, and a purpose-made interface.

## Modules
### Sequencer
Handles all the timing and triggering of sounds. Used the rodio library beneath the hood with a custom audio source that keeps samples in memory and really reduces latency. Command processing and sound playing run in their own threads, with handles provided to modify properties. Current latency is in the tens of microseconds on modern hardware. I'm aiming to keep this below the threshold of human perception (10ms or so)

### Display
Handles displaying sequencer state. I'm working on a CLI interface, but this could be extended to, say, a microcontroller OLED display with light up buttons. It is part of the Controller module, which handles writing to it.

### Controller
The sequencer is controlled by the aptly named Controller via message passing. The sequencer writes to a message channel its state, and receives commands via a command channel.

## Future work
To be added is an input module and a MIDI module. This way the drum machine can start to be used as an actual instrument.
