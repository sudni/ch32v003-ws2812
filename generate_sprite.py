import math

width, height = 32, 32
sprite_data = []
for y in range(height):
    for x in range(width):
        # Create a simple pattern, e.g. a circle with a gradient
        dx = x - width/2 + 0.5
        dy = y - height/2 + 0.5
        r = math.sqrt(dx*dx + dy*dy)
        if r < 14:
            # Color based on radius and angle
            angle = math.atan2(dy, dx)
            # R, G, B
            red = int(math.sin(angle) * 15 + 15)
            green = int(math.cos(angle) * 31 + 31)
            blue = int(math.cos(angle * 2) * 15 + 15)
            # clamp
            red = max(0, min(31, red))
            green = max(0, min(63, green))
            blue = max(0, min(31, blue))
            color16 = (red << 11) | (green << 5) | blue
        else:
            color16 = 0x0000 # black or transparent
        
        # We need to store MSB first for the SPI transfer
        msb = (color16 >> 8) & 0xFF
        lsb = color16 & 0xFF
        sprite_data.append(msb)
        sprite_data.append(lsb)

print(f"pub const SPRITE_32X32: [u8; {len(sprite_data)}] = {sprite_data};")
