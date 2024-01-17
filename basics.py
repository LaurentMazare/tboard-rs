import tboard

tb = tboard.EventWriter("/tmp/test-event-writer")
tb.add_scalar("sample-tag", 3.14159265358979, 42)
tb.flush()

print(tb.filename)

event_reader = tboard.EventReader(tb.filename)
for event in event_reader:
    print(event)
