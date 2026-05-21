# connect-group-schedules.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/connect-group-schedules.ttl`
- **Triples:** 300
- **Classes:** 3 · **Properties:** 17 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `GroupMeeting` | Group Meeting | An individual meeting instance of a Connect Group, derived from the GroupSchedule. Records the actual meeting date, star |
| `GroupSchedule` | Group Schedule | The recurring meeting schedule for a Connect Group, declaring the day of week, start time, frequency, and location mode. |
| `GroupSeason` | Group Season | A bounded active season for a Connect Group — semester, annual cycle, or ongoing. Seasons control the time window in whi |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `meetingDate` | GroupMeeting | date | ISO-8601 calendar date of the individual meeting instance. |
| `meetingDerivedFromSchedule` | GroupMeeting | GroupSchedule | Links the meeting instance back to the GroupSchedule from which it was generated |
| `meetingForGroup` | GroupMeeting | ConnectGroup | Associates the GroupMeeting with its Connect Group. |
| `meetingIsCancelled` | GroupMeeting | boolean | True when this specific meeting instance has been cancelled. Cancelled meetings  |
| `meetingLocation` | GroupMeeting | string | Human-readable address or platform description for this meeting instance, e.g. ' |
| `meetingStartTime` | GroupMeeting | time | ISO-8601 time-of-day at which this specific meeting instance begins. |
| `scheduleDayOfWeek` | GroupSchedule | string | Day of the week the group meets, as a string matching a DayOfWeekScheme notation |
| `scheduleDayTerm` | GroupSchedule | Concept | SKOS concept from zoe:DayOfWeekScheme for the meeting day, enabling vocabulary-d |
| `scheduleForGroup` | GroupSchedule | ConnectGroup | Associates the GroupSchedule with its Connect Group. |
| `scheduleFrequency` | GroupSchedule | string | Recurrence cadence for the group meeting. Allowed values: 'weekly', 'biweekly',  |
| `scheduleLocationMode` | GroupSchedule | Concept | SKOS concept from zoe:LocationModeScheme indicating whether this group meets in- |
| `scheduleTime` | GroupSchedule | string | Start time of the recurring meeting in local 24-hour format (HH:MM), e.g. '19:00 |
| `seasonEndDate` | GroupSeason | date | ISO-8601 date on which the season closes. GroupMeetings scheduled after this dat |
| `seasonForGroup` | GroupSeason | ConnectGroup | Associates the GroupSeason with the Connect Group it governs. |
| `seasonIsActive` | GroupSeason | boolean | True when the season is currently open. Controls whether new GroupMeetings may b |
| `seasonLabel` | GroupSeason | string | Human-readable name for the season, e.g. 'Spring 2026' or 'Fall Semester 2026'. |
| `seasonStartDate` | GroupSeason | date | ISO-8601 date on which the season becomes active. GroupMeetings scheduled before |
