import math
import tboard

tb = tboard.EventWriter("/tmp/test-event-writer")
for step in range(100000):
    tb.add_scalar("sin(step)", math.sin(step * 1e-4), step)
tb.flush()

print(tb.filename)

event_reader = tboard.EventReader(tb.filename)
for event in event_reader:
    print(event)
