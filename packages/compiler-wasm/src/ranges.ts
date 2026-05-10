export interface ByteRange {
  readonly start: number;
  readonly end: number;
}

export interface Utf16OffsetRange {
  readonly start: number;
  readonly end: number;
}

export interface Utf16Position {
  readonly line: number;
  readonly character: number;
  readonly offset: number;
}

export interface Utf16LineColumnRange {
  readonly start: Utf16Position;
  readonly end: Utf16Position;
}

interface OffsetCursor {
  readonly byteOffset: number;
  readonly utf16Offset: number;
  readonly line: number;
  readonly character: number;
}

export function byteRangeToUtf16Range(source: string, range: ByteRange): Utf16OffsetRange {
  assertOrderedRange(range);

  return {
    start: byteOffsetToUtf16Offset(source, range.start),
    end: byteOffsetToUtf16Offset(source, range.end)
  };
}

export function byteRangeToUtf16LineColumnRange(
  source: string,
  range: ByteRange
): Utf16LineColumnRange {
  assertOrderedRange(range);

  return {
    start: byteOffsetToUtf16Position(source, range.start),
    end: byteOffsetToUtf16Position(source, range.end)
  };
}

export function byteOffsetToUtf16Offset(source: string, byteOffset: number): number {
  return cursorAtByteOffset(source, byteOffset).utf16Offset;
}

export function byteOffsetToUtf16Position(source: string, byteOffset: number): Utf16Position {
  const cursor = cursorAtByteOffset(source, byteOffset);

  return {
    line: cursor.line,
    character: cursor.character,
    offset: cursor.utf16Offset
  };
}

function cursorAtByteOffset(source: string, targetByteOffset: number): OffsetCursor {
  assertByteOffset(targetByteOffset);

  let byteOffset = 0;
  let utf16Offset = 0;
  let line = 0;
  let character = 0;

  if (targetByteOffset === 0) {
    return { byteOffset, utf16Offset, line, character };
  }

  for (const codePoint of source) {
    const nextByteOffset = byteOffset + utf8ByteLength(codePoint);
    const nextUtf16Offset = utf16Offset + codePoint.length;
    const nextLine = codePoint === "\n" ? line + 1 : line;
    const nextCharacter = codePoint === "\n" ? 0 : character + codePoint.length;

    if (targetByteOffset === nextByteOffset) {
      return {
        byteOffset: nextByteOffset,
        utf16Offset: nextUtf16Offset,
        line: nextLine,
        character: nextCharacter
      };
    }

    if (targetByteOffset < nextByteOffset) {
      throw new RangeError(`Byte offset ${targetByteOffset} is not aligned to a UTF-8 boundary.`);
    }

    byteOffset = nextByteOffset;
    utf16Offset = nextUtf16Offset;
    line = nextLine;
    character = nextCharacter;
  }

  if (targetByteOffset === byteOffset) {
    return { byteOffset, utf16Offset, line, character };
  }

  throw new RangeError(
    `Byte offset ${targetByteOffset} is outside the source byte length ${byteOffset}.`
  );
}

function assertOrderedRange(range: ByteRange): void {
  assertByteOffset(range.start);
  assertByteOffset(range.end);

  if (range.start > range.end) {
    throw new RangeError(`Byte range start ${range.start} must not exceed end ${range.end}.`);
  }
}

function assertByteOffset(byteOffset: number): void {
  if (!Number.isSafeInteger(byteOffset) || byteOffset < 0) {
    throw new RangeError(`Byte offset ${byteOffset} must be a non-negative safe integer.`);
  }
}

function utf8ByteLength(codePoint: string): number {
  const value = codePoint.codePointAt(0);

  if (value === undefined) {
    return 0;
  }

  if (value <= 0x7f) {
    return 1;
  }
  if (value <= 0x7ff) {
    return 2;
  }
  if (value <= 0xffff) {
    return 3;
  }
  return 4;
}
