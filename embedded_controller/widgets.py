from __future__ import annotations
from PIL import ImageDraw, ImageFont
from abc import ABC, abstractmethod

class Widget(ABC):
    _font = ImageFont.load_default()

    def __init__(self, children: list[Widget] = []):
        self.children = children

    def add_child(self, child: Widget):
        self.children.append(child)

    @abstractmethod
    def get_size(self) -> tuple[int, int]:
        pass

    @abstractmethod
    def render(self, draw: ImageDraw, x: int, y: int):
        pass

    
class HLayout(Widget):
    """
        Horizontal layout container widget
        Renders children left to right, adjusting its size dynamically
    """
    def __init__(self, children: list[Widget] = [], spacing: int = 2, border=False):
        self._width: int = 0
        self._height: int = 0
        self._spacing = spacing
        self._border = border
        super().__init__(children)
    
    def render(self, draw: ImageDraw, x: int, y: int):
        width: int = self._spacing
        height: int = 0
        for child in self.children:
            child.render(draw, x + width, y + self._spacing)
            w, h = child.get_size()
            width += w + self._spacing
            height = max(height, h + self._spacing)
        self._width = width
        self._height = height

        if self._border:
            draw.rectangle((x, y, x + width, y + height), outline=255)
    
    def get_size(self) -> tuple[int, int]:
        return self._width, self._height

class VLayout(Widget):
    """
        Vertical layout container widget
        Renders children top to bottom, adjusting its size dynamically
    """
    def __init__(self, children: list[Widget] = [], spacing: int = 2, border=False):
        self._width: int = 0
        self._height: int = 0
        self._spacing = spacing
        self._border = border
        super().__init__(children)
    
    def render(self, draw: ImageDraw, x: int, y: int):
        height: int = self._spacing
        width: int = 0
        for child in self.children:
            child.render(draw, x + self._spacing, y + height)
            w, h = child.get_size()
            height += h + self._spacing
            width = max(width, w + 2*self._spacing)
        self._width = width
        self._height = height

        if self._border:
            draw.rectangle((x, y, x + width, y + height), outline=255)
    
    def get_size(self) -> tuple[int, int]:
        return self._width, self._height

class Text(Widget):
    """
        Text widget
        Renders a string of text
    """
    def __init__(self, text: str = ""):
        self.text = text
        super().__init__()
    
    def render(self, draw: ImageDraw, x: int, y: int):
        draw.text(
            (x, y),
            f"{self.text}",
            font=self._font,
            fill=255
        )
    
    def get_size(self) -> tuple[int, int]:
        bbox: list[int] = self._font.getbbox(self.text)
        return bbox[2] - bbox[0] + 2, bbox[3] - bbox[1] + 4 # default 2 padding