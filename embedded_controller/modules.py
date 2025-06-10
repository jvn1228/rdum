from typing_extensions import ParamSpecKwargs
from widgets import *
from PIL import ImageDraw, ImageFont
from abc import ABC, abstractmethod
from zmq_channel import State
from dataclasses import dataclass, field
import util

@dataclass
class UIState:
    enc1_pos: int = 0
    enc1_d: int = 0
    enc2_pos: int = 0
    enc2_d: int = 0
    button1_pressed: bool = False
    button2_pressed: bool = False
    pad_values: list[int] = field(default_factory=list)

class Module(ABC):
    @abstractmethod
    def render_primary(self, draw: ImageDraw):
        pass
    
    @abstractmethod
    def render_secondary(self, draw: ImageDraw):
        pass
    
    @abstractmethod
    def on_state_update(self):
        pass

class Status(Module):
    def __init__(self, ui_state: UIState):
        super().__init__()
        self._ui_state = ui_state

        self._ip_text: Text = Text(util.get_ip())
        self._tracks_text: Text = Text("0")
        self._enc1_pos_text: Text = Text(str(self._ui_state.enc1_pos))
        self._enc2_pos_text: Text = Text(str(self._ui_state.enc2_pos))
        self._button1_text: Text = Text(str(self._ui_state.button1_pressed))
        self._button2_text: Text = Text(str(self._ui_state.button2_pressed))
        self._pad_values_text1: Text = Text(str(self._ui_state.pad_values[:4]))
        self._pad_values_text2: Text = Text(str(self._ui_state.pad_values[4:]))
        
        self._primary_widget = VLayout([
            HLayout([Text("Enc1"), self._enc1_pos_text, Text("Enc2"), self._enc2_pos_text]),
            HLayout([Text("Btn1"), self._button1_text, Text("Btn2"), self._button2_text]),
            HLayout([Text("Pad Values"), self._pad_values_text1]),
            HLayout([self._pad_values_text2])
        ], border=True)
        self._secondary_widget = VLayout([
            HLayout([
                Text("IP:"),
                self._ip_text,
            ]),
            HLayout([
                Text("Tracks:"),
                self._tracks_text,
            ])
        ], border=True)
    
    def render_primary(self, draw: ImageDraw):
        self._primary_widget.render(draw, 0, 0)
    
    def render_secondary(self, draw: ImageDraw):
        self._secondary_widget.render(draw, 0, 0)
    
    def on_state_update(self, state: State):
        self._tracks_text.text = str(len(state.trks))

    def on_input_update(self):
        self._ip_text.text = util.get_ip()
        self._enc1_pos_text.text = str(self._ui_state.enc1_pos)
        self._enc2_pos_text.text = str(self._ui_state.enc2_pos)
        self._button1_text.text = str(self._ui_state.button1_pressed)
        self._button2_text.text = str(self._ui_state.button2_pressed)
        self._pad_values_text1.text = str(self._ui_state.pad_values[:4])
        self._pad_values_text2.text = str(self._ui_state.pad_values[4:])

class Playback(Module):
    def __init__(self):
        super().__init__()
        self._last_state = State()
        self._font = ImageFont.load_default()
    
    def on_state_update(self, state: State):
        self._last_state = state

    def on_input_update(self):
        pass
    
    def render_primary(self, draw: ImageDraw):
        width = 128
        height = 64
        len = self._last_state.trks[0].len
        # Perceived sync is better with a leading idx
        trk_idx = (self._last_state.trks[0].idx) % len
        # Display track_idx in header
        header_text = f"{trk_idx+1 if trk_idx < len else len}"
        bbox: list[int] = self._font.getbbox(header_text)
        text_width, text_height = bbox[2] - bbox[0], bbox[3] - bbox[1]
        draw.text(
            (5, 5),
            header_text,
            font=self._font,
            fill=255
        )
        
        # Draw a separator line
        if trk_idx > 0:
            draw.line([(0, text_height + 10), (width * trk_idx // (len-1), text_height + 10)], fill=255)
        
        # Calculate area for progress bars
        progress_start_y = text_height + 15
        progress_height = 10
        progress_spacing = 5
        label_width = 10
        progress_width = width - label_width - 5  # 5 pixels from right edge
        
        # Draw progress bars for each track
        for i, track in enumerate(self._last_state.trks):
            y_pos = progress_start_y + i * (progress_height + progress_spacing)
            
            # Draw track label (first letter of track name)
            track_label = track.name[0]  # First letter of track name
            bbox = self._font.getbbox(track_label)
            draw.text(
                (5, y_pos + (progress_height - text_height) // 2),
                track_label,
                font=self._font,
                fill=255
            )
            
            # # Draw track progress bar outline
            # self._draw.rectangle(
            #     (label_width + 5, y_pos, label_width + 5 + progress_width, y_pos + progress_height),
            #     outline=255,
            #     fill=0
            # )
            
            # Draw progress bar segments based on pattern
            segment_width = progress_width / len
            for j, val in enumerate(track.slots):
                if val > 0:  # If this step is active
                    segment_x = label_width + 5 + j * segment_width
                    draw.rectangle(
                        (segment_x, y_pos, segment_x + segment_width, y_pos + progress_height),
                        outline=0,
                        fill=255
                    )
            
        # Highlight current position in pattern
        cursor_x = label_width + 5 + trk_idx * segment_width
        cursor_height = (progress_height + progress_spacing) * 3
        draw.rectangle(
            (cursor_x, progress_start_y - 1, cursor_x + segment_width, progress_start_y + cursor_height - 1),
            outline=255,
            fill=None
        )
    
    def render_secondary(self, draw: ImageDraw):
        pass
        