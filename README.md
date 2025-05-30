# RDUM
Rust-based drum machine

## Motivation
Part of my learning process for any language must include making musical things. I'm not a drummer, and I already spend too much on instruments to buy another one. Luckily my computer keeps pretty good time. Except now I'm making this instead of jamming! Some interesting challenges that arise when making any digital instrument:
- There is a very limited amount of time to process before we must play the next audio frame otherwise we get stuttering and other unpleasant artifacts
- Timing has to be precise because too much latency or drift can really ruin playing along with other instruments
- Interacting with the instrument should ideally be pleasant and intuitive (at least for me, no doubt the primary user of this thing)
I'm just playing around with Rust so we'll see how far we get, but one day this can perhaps run on a Raspberry Pi and be completely independent, with MIDI clock and line outs, and a purpose-made interface.

My RPi came in and since it's just a very small Linux computer, it can maybe one day do fancy things that no mere off the shelf drum machine can do. Play along with an audio input? Add variations on the fly? Connect to my phone for further control?

## Modules
### Sequencer
Handles all the timing and triggering of sounds. Used the rodio library beneath the hood with a custom audio source that keeps samples in memory and really reduces latency. Command processing and sound playing run in their own threads, with handles provided to modify properties. Current latency is at most a few microseconds, even on the Pi.

### Controller
The sequencer is controlled by the aptly named Controller via message passing. The sequencer writes to a message channel its state, and receives commands via a command channel. Controller also handles displaying the states.

Examples of controllers: CLI, hardware interface layer, web site 

I'm currently working on a controller for the Raspberry Pi using CircuitPython libraries to interface with the hardware. The interprocess communication can be handled by the intermediate ZeroMQ controller and protobuf messages. Because I come from a platform engineering background. It should still be fast enough!

Also, I used Claude for that webUI stuff so don't pay attention to that. I just needed something quick so I can do the fun stuff.

## Future work
To be added is an input module and a MIDI module. This way the drum machine can start to be used as an actual instrument.
