"""Generate packed field metadata from EA's user-supplied F1 25 structures.

Usage: python scripts/generate-f1-fields.py structures.txt assets/f1-2025-fields.json
No compiler, network access, or execution of the input document is involved.
"""
import json
import re
import sys
from pathlib import Path

PRIMITIVES = {"uint8": 1, "int8": 1, "char": 1, "uint16": 2, "int16": 2,
              "uint32": 4, "uint": 4, "NEuint": 4, "NEuint8": 1,
              "uint64": 8, "float": 4, "double": 8}
PACKETS = ["Motion", "Session", "Lap", "Event", "Participants", "CarSetup",
           "CarTelemetry", "CarStatus", "FinalClassification", "LobbyInfo",
           "CarDamage", "SessionHistory", "TyreSets", "MotionEx", "TimeTrial",
           "LapPositions"]
GROUPS = ["Motion", "Session", "Lap", "Event", "Participants", "Setup",
          "Telemetry", "Status", "Classification", "Lobby", "Damage",
          "History", "Tyre sets", "Motion Ex", "Time trial", "Lap positions"]
EXPECTED_SIZES = [1349, 753, 1285, 45, 1284, 1133, 1352, 1239,
                  1042, 954, 1041, 1460, 231, 273, 101, 1131]


def words(name):
    name = name.removeprefix("m_")
    label = re.sub(r"(?<=[a-z0-9])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])", " ", name)
    return label[0].upper() + label[1:]


def unit(name, comment):
    c = comment.lower()
    if "milliseconds" in c or name.endswith(("InMS", "MSPart")):
        return "ms"
    if "minutes part" in c:
        return "min"
    if "percentage" in c or "percent" in name.lower():
        return "%"
    if "celsius" in c:
        return "°C"
    if "psi" in c:
        return "PSI"
    if "joules" in c or name in {"ersStoreEnergy", "ersHarvestedThisLapMGUK", "ersHarvestedThisLapMGUH", "ersDeployedThisLap"}:
        return "J"
    if "(w)" in c:
        return "W"
    if "radians" in c or "radia" in c:
        return "rad"
    if "kilometres per hour" in c or "kmph" in c:
        return "km/h"
    if "metres/s" in c:
        return "m/s"
    if "metres" in c or re.search(r"\bin m\b", c):
        return "m"
    if "seconds" in c:
        return "s"
    if name in {"fuelInTank", "fuelCapacity"}:
        return "kg"
    if "rpm" in name.lower():
        return "RPM"
    if "gforce" in name.lower():
        return "G"
    return ""


def generate(source):
    constants = {}
    for name, expression in re.findall(r"static const \w+\s+(\w+)\s*=\s*([\d +]+);", source):
        constants[name] = sum(int(x) for x in expression.split("+"))

    def count(expression):
        return constants[expression] if expression in constants else int(expression)

    # Named top-level structs have no nested declarations except Event, handled below.
    structures = {}
    for match in re.finditer(r"^struct (\w+)\s*\{(.*?)^\};", source, re.M | re.S):
        members = []
        for line in match[2].splitlines():
            field = re.match(r"\s*(\w+)\s+(\w+)\s*((?:\[\w+\])*)\s*;\s*(?://(.*))?", line)
            if field:
                members.append((field[1], field[2].removeprefix("m_"), [count(x) for x in re.findall(r"\[(\w+)\]", field[3])], (field[4] or "").strip()))
        structures[match[1]] = members

    # Event union members occupy the same address; only decode the matching event.
    union = re.search(r"union EventDataDetails\s*\{(.*?)^\};", source, re.S | re.M)[1]
    events = {}
    for body, name in re.findall(r"struct\s*\{(.*?)\}\s*(\w+)\s*;", union, re.S):
        fields = []
        for kind, member, comment in re.findall(r"(\w+)\s+(\w+)\s*;\s*//([^\n]*)", body):
            fields.append((kind, member, [], comment.strip()))
        events[name] = fields

    def size(kind):
        if kind in PRIMITIVES:
            return PRIMITIVES[kind]
        if kind == "EventDataDetails":
            return 12  # fixed 45-byte packet including header and event code
        return sum(size(k) * product(dims) for k, _, dims, _ in structures[kind])

    def product(dims):
        value = 1
        for d in dims:
            value *= d
        return value

    event_codes = {"FastestLap": "FTLP", "Retirement": "RTMT", "DRSDisabled": "DRSD",
                   "TeamMateInPits": "TMPT", "RaceWinner": "RCWN", "Penalty": "PENA",
                   "SpeedTrap": "SPTP", "StartLights": "STLG", "DriveThroughPenaltyServed": "DTSV",
                   "StopGoPenaltyServed": "SGSV", "Flashback": "FLBK", "Buttons": "BUTN",
                   "Overtake": "OVTK", "SafetyCar": "SCAR", "Collision": "COLL"}
    results = []
    for packet_id, (packet, group) in enumerate(zip(PACKETS, GROUPS)):
        fields = []

        def walk(kind, offset, prefix, dims, condition=None):
            cursor = offset
            members = structures[kind]
            offsets = {}
            for k, name, lengths, _ in members:
                offsets[name] = cursor
                cursor += size(k) * product(lengths)
            cursor = offset
            for k, name, lengths, comment in members:
                field_size = size(k) * product(lengths)
                if name == "header":
                    cursor += field_size
                    continue
                path = prefix + [name]
                dimensions = list(dims)
                if k != "char":
                    for i, length in enumerate(lengths):
                        dim = {"count": length, "stride": size(k) * product(lengths[i+1:]), "label": words(name)}
                        if length == 22 and (name in {"carMotionData", "lapData", "participants", "carSetups", "carTelemetryData", "carStatusData", "classificationData", "lobbyPlayers", "carDamageData"} or name == "positionForVehicleIdx" and i == 1):
                            dim["player"] = True
                            dim["label"] = "Car"
                        elif length == 4 and k in PRIMITIVES:
                            dim["label"] = "Wheel (RL / RR / FL / FR)"
                        count_fields = {"marshalZones": "numMarshalZones", "weatherForecastSamples": "numWeatherForecastSamples", "weekendStructure": "numSessionsInWeekend", "participants": "numActiveCars", "classificationData": "numCars", "lobbyPlayers": "numPlayers", "lapHistoryData": "numLaps", "tyreStintsHistoryData": "numTyreStints", "positionForVehicleIdx": "numLaps", "liveryColours": "numColours"}
                        if name in count_fields and i == 0:
                            dim["valid_count_offset"] = offsets[count_fields[name]]
                        dimensions.append(dim)
                if k in PRIMITIVES:
                    # A NUL-terminated character array is one text field.
                    field = {"key": "f1." + packet[0].lower() + packet[1:] + "." + ".".join(path),
                             "label": group + " · " + " / ".join(words(p) for p in path if p not in {"carMotionData", "lapData", "participants", "carSetups", "carTelemetryData", "carStatusData", "classificationData", "lobbyPlayers", "carDamageData", "eventDetails"}),
                             "offset": cursor, "kind": k, "dimensions": dimensions,
                             "unit": unit(name, comment), "description": comment}
                    if k == "char":
                        field["text_len"] = product(lengths)
                    if condition:
                        field["event"] = condition
                    # Enumerations documented entirely in this comment. Unknown values stay numeric.
                    field["states"] = {code: label.strip() for code, label in re.findall(r"(-?\d+)\s*=\s*([^,]+)", comment)}
                    fields.append(field)
                elif k == "EventDataDetails":
                    for event_name, event_members in events.items():
                        structures["Event" + event_name] = event_members
                        walk("Event" + event_name, cursor, path + [event_name], dimensions, event_codes[event_name])
                else:
                    walk(k, cursor, path, dimensions, condition)
                cursor += field_size

        struct_name = "Packet" + packet + "Data"
        walk(struct_name, 0, [], [])
        # Include the event code as a string rather than four numeric byte widgets.
        if packet_id == 3:
            fields[0]["kind"] = "char"
            fields[0]["text_len"] = 4
            fields[0]["dimensions"] = []
        packet_size = size(struct_name)
        if packet_size != EXPECTED_SIZES[packet_id]:
            raise ValueError(f"{packet}: calculated {packet_size} bytes, expected {EXPECTED_SIZES[packet_id]}")
        results.append({"id": packet_id, "size": packet_size, "group": group, "fields": fields})
    return results


if __name__ == "__main__":
    metadata = generate(Path(sys.argv[1]).read_text(encoding="utf-8-sig"))
    Path(sys.argv[2]).write_text(json.dumps(metadata, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    for packet in metadata:
        print(f'{packet["id"]:2}: {packet["group"]:16} {packet["size"]:4} bytes, {len(packet["fields"])} fields')
