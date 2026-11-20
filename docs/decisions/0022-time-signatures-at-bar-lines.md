# 0022. Change time signatures only at bar lines and use the lower number for the beat

- Status: Accepted
- Date: 2026-11-20

## Context

The arranger, the snap grid, and the clip launcher need bars and beats. Record 0021 counts musical time in ticks of the quarter note, and it uses the word "beat" for the quarter note. A time signature sets the bars and beats, but its beat is not always a quarter note.

## Decision

- A time signature has an upper number and a lower number. The upper number is the number of beats in a bar. The lower number is the note value of one beat.
- The upper number is 1 to 99. The lower number is 1, 2, 4, 8, 16, 32, or 64.
- A beat is the note value of the lower number. 6/8 has six beats, and each beat is an 8th note.
- The quarter note stays the unit of ticks and tempo. The code calls this unit a quarter note, not a beat. This replaces the word "beat" in record 0021.
- A time signature change always starts a new bar. Each change belongs to a bar number. No bar is shorter than its time signature.
- Bar 1 starts at the start of the project. Before bar 1, the first time signature continues. The bars before bar 1 have the numbers 0, -1, -2, and lower.
- Bar numbers and beat numbers start at 1. In a beat, the offset counts ticks from 0.
- The code calls a time signature `TimeSignature`, because the word "meter" also names a level meter.

## Consequences

- If you change the time signature of an earlier bar, the later changes stay at their bar numbers. Their positions in ticks move.
- Clips keep their positions in ticks. A change to an earlier bar can move the bar lines under a clip.
- Bar lines and beats are at exact ticks. A conversion between ticks and bars and beats does not round.
- A count-in or a pickup can use bar 0.
- Beat groups such as 2+2+3 are not available. They can come later for metronome accents.

## Alternatives

- Changes at all positions: more freedom for audio without a grid, but the grid and the bar numbers must show short bars.
- A beat that follows the felt pulse: 6/8 has two beats of a dotted quarter note, but each odd time signature needs a rule for its groups.
- A beat that is always a quarter note: one meaning of the word "beat", but a bar of 7/8 ends on half a beat.
- The name `Meter`: shorter, but the word also names the level meters of the mixer.
