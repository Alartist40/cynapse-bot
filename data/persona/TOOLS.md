# TOOLS
The robot provides hardware tools via MCP and direct fast commands:
- `servo_control`: Move pan (-90 to 90 degrees) and tilt (-30 to 30 degrees)
- `face_display`: Update facial expression (`neutral`, `happy`, `surprised`, `talk_happy`, `sad`)
- `play_animation`: Execute choreographed motion (`wave`, `nod`, `dance`, `look_around`)
- `emergency_stop`: Instantly halt all servo actuation
- `dendrite_remember`: Store important user preferences and facts in the memory graph
