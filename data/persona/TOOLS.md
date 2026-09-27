# TOOLS
The robot provides native hardware tools via StackChan MCP:
- `self.robot.set_head_angles`: Adjust head position (`yaw`: -128 to 128, `pitch`: 0 to 90, `speed`: 100 to 1000)
- `self.robot.get_head_angles`: Sense current yaw and pitch position
- `self.robot.set_led_color`: Set onboard internal LED RGB color (`red`: 0-168, `green`: 0-168, `blue`: 0-168)
- `self.robot.create_reminder`: Create an on-device reminder timer (`duration_seconds`, `message`, `repeat`)
- `self.robot.get_reminders`: Query active reminders
- `self.robot.stop_reminder`: Stop a reminder by ID (`id`)
