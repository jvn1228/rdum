use crossterm::{                                                                                                                                                                                                                                
    execute,                                                                                                                                  
    terminal,
    cursor,
    ExecutableCommand                                                                     
};
use std::error::Error;
use std::io;
use crate::sequencer;

pub trait Display {
    fn write_state(&mut self, s: sequencer::State) -> Result<(), Box<dyn Error>>;
}

pub struct CLIDisplay {
    stdout: io::Stdout,
}

impl CLIDisplay {
    pub fn new() -> Result<CLIDisplay, Box<dyn Error>> {
        let mut stdout = io::stdout();
        stdout.execute(cursor::Hide)?;
        Ok(CLIDisplay {
            stdout
        })
    }
}

impl Display for CLIDisplay {
    fn write_state(&mut self, s: sequencer::State) -> Result<(), Box<dyn Error>> {
        self.stdout.execute(terminal::Clear(terminal::ClearType::All))?;
        self.stdout.execute(cursor::MoveTo(0,0))?;

        let name_offset: usize = s.trks.iter().map(|t| { t.name.len() }).max().unwrap();
        for trk in &s.trks {
            if trk.name.len() < name_offset {
                self.stdout.execute(cursor::MoveRight((name_offset - trk.name.len()) as u16))?;
            }
            print!("{}", trk.name);
            self.stdout.execute(cursor::MoveToNextLine(1))?;
        }

        self.stdout.execute(cursor::MoveTo(0,0))?;

        for slot in 0..s.len {
            for (row, trk) in s.trks.iter().enumerate() {
                self.stdout.execute(cursor::MoveTo((slot + name_offset) as u16 + 1, row as u16))?;
                // Setting the idx back by 1 aligns the eye and ears perceptually better
                // that is, the jump from slot 1 -> 2 is when the 2 sound hits
                if slot == (s.trk_idx + s.len - 1) % s.len {
                    print!("O");
                } else if trk.slots[slot] > 0 {
                    print!("X");
                } else {
                    print!("_");
                }
            }
        }
        
        self.stdout.execute(cursor::MoveToNextLine(2))?;
        print!("Tempo: {} bpm", s.tempo);
        self.stdout.execute(cursor::MoveToNextLine(1))?;
        print!("Division: 1/{}", s.division);
        self.stdout.execute(cursor::MoveToNextLine(1))?;
        print!("Accounting for latency: {:?}", s.latency);
        self.stdout.execute(cursor::MoveToNextLine(1))?;

        Ok(())
    }
}

pub struct MockDisplay {}

impl Display for MockDisplay {
    fn write_state(&mut self, s: sequencer::State) -> Result<(), Box<dyn Error>> {
        execute!(io::stdout(), cursor::MoveTo(0,0))?;
        println!("{:?}", s);
        Ok(())    
    }
}