1. Parse MIDI -> Internal Song Model
take midi data and turn file data into internal model
2. Prepare Events for Playback
schedule all music data st specific times based on the internal model, so
we'll send these bytes at this time
3. Playback Engine (Scheduler / Transport) (can probably be vibe coded)
hooks for ui with play, pause, restart, pick a position, etc
4. OS MIDI Backend (Syscalls / API Layer)
interface for the playback engine to use so that it can send data to the os for playback

we should do this in the order probably

2, 1, 4, 3