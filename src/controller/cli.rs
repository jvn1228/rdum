use crate::sequencer;
use std::sync::mpsc;
use std::time::{Instant, Duration};

use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::widgets;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
    DefaultTerminal, Frame,
};

#[derive(Debug)]
pub struct CLIController {
    state_rx: mpsc::Receiver<sequencer::StateUpdate>,
    cmd_tx: mpsc::Sender<sequencer::Command>,
    exit: bool,
    refresh_interval: Duration,
    last_refresh: Instant,
    last_state: sequencer::SeqState,
}

impl CLIController {
    pub fn new(rx: mpsc::Receiver<sequencer::StateUpdate>, tx: mpsc::Sender<sequencer::Command>) -> Self {
        CLIController {
            state_rx: rx,
            cmd_tx: tx,
            exit: false,
            refresh_interval: Duration::from_secs_f32(1.0/12.0),
            last_refresh: Instant::now(),
            last_state: sequencer::SeqState::default()
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            let now = Instant::now();
            if let Ok(state) = self.state_rx.try_recv() {
                match state {
                    sequencer::StateUpdate::SeqState(state) => self.last_state = state,
                    _ => {}
                }
            }
            if now.duration_since(self.last_refresh) > self.refresh_interval {
                terminal.draw(|frame| self.draw(frame))?;
                self.last_refresh = now;
            }
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn send_play_sample_cmd(&self, c: char) {
        let c = c.to_digit(10).unwrap_or(0) as usize;
        if c < self.last_state.trks.len() {
            self.cmd_tx.send(sequencer::Command::PlaySound(c, 127)).expect("Bad play command")
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char(c) if c.is_digit(10) => self.send_play_sample_cmd(c),
            KeyCode::Char('p') => self.cmd_tx.send(if self.last_state.playing { sequencer::Command::StopSequencer } else { sequencer::Command::PlaySequencer }).expect("Bad stuff"),
            _ => {}
        }
    }

    /// updates the application's state based on user input
    fn handle_events(&mut self) -> io::Result<()> {
        if let Ok(is_event) = event::poll(Duration::ZERO) {
            if is_event {
                match event::read()? {
                    // it's important to check that the event is a key press event as
                    // crossterm also emits key release and repeat events on Windows.
                    Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                        self.handle_key_event(key_event)
                    },
                    _ => {}
                };
            }
        }
        Ok(())
    }
}

impl Widget for &CLIController {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" Rdum ".bold());
        let instructions = Line::from(vec![
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let something = format!("{:?}", self.last_state).to_string();

        let text = Text::from(something);

        Paragraph::new(text)
            .centered()
            .block(block)
            .wrap(widgets::Wrap{ trim: true })
            .render(area, buf);
    }
}