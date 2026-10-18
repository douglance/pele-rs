import Foundation

extension Pele {
    public struct CivilDateTime: Codable, Sendable {
        public var year: Int, month: Int, day: Int, hour: Int, minute: Int, second: Int, millisecond: Int
        public init(year: Int, month: Int, day: Int, hour: Int = 0, minute: Int = 0, second: Int = 0, millisecond: Int = 0) {
            self.year = year; self.month = month; self.day = day; self.hour = hour
            self.minute = minute; self.second = second; self.millisecond = millisecond
        }
    }
    public struct CalendarFields: Codable, Sendable {
        public var civil: CivilDateTime
        /// Sunday is zero. Offset minutes are east of UTC.
        public var weekday: Int, utcOffsetMinutes: Int
        public init(civil: CivilDateTime, weekday: Int, utcOffsetMinutes: Int) {
            self.civil = civil; self.weekday = weekday; self.utcOffsetMinutes = utcOffsetMinutes
        }
    }
}
