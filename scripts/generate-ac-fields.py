"""Read ctypes declarations as data (AST only); generate the AC shared-memory catalog.

Source: ac-custom-shaders-patch/acc-extension-apps,
apps/python/AccExtHelper/sim_info.py (Rombik's shared memory declarations).
Usage: python scripts/generate-ac-fields.py sim_info.py assets/ac-fields.json
Windows wchar_t is always UTF-16 (two bytes), irrespective of the generator OS.
"""
import ast
import json
import re
import sys
from pathlib import Path


def label(name):
    text = re.sub(r"(?<=[a-z0-9])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])", " ", name.strip("_"))
    return text[0].upper() + text[1:]


def shape(node):
    if isinstance(node, ast.Name):
        return node.id, []
    if isinstance(node, ast.BinOp) and isinstance(node.op, ast.Mult) and isinstance(node.right, ast.Constant) and isinstance(node.right.value, int):
        kind, lengths = shape(node.left)
        return kind, [node.right.value] + lengths
    raise ValueError("Unsupported ctypes declaration")


UNITS = {
    "gas": "%", "brake": "%", "clutch": "%", "kersCharge": "%", "kersInput": "%", "brakeBias": "%",
    "fuel": "L", "maxFuel": "L", "speedKmh": "km/h", "rpms": "RPM", "maxRpm": "RPM",
    "accG": "G", "velocity": "m/s", "localVelocity": "m/s", "wheelAngularSpeed": "rad/s",
    "camberRAD": "rad", "heading": "rad", "pitch": "rad", "roll": "rad", "steerAngle": "rad",
    "wheelsPressure": "PSI", "wheelLoad": "N", "tyreCoreTemperature": "°C", "brakeTemp": "°C",
    "tyreTempI": "°C", "tyreTempM": "°C", "tyreTempO": "°C", "airTemp": "°C", "roadTemp": "°C",
    "suspensionTravel": "m", "cgHeight": "m", "rideHeight": "m", "tyreRadius": "m", "suspensionMaxTravel": "m",
    "distanceTraveled": "m", "trackSPlineLength": "m", "carCoordinates": "m", "tyreContactPoint": "m",
    "iCurrentTime": "ms", "iLastTime": "ms", "iBestTime": "ms", "lastSectorTime": "ms", "sessionTimeLeft": "ms",
    "kersCurrentKJ": "kJ", "kersMaxJ": "J", "ersMaxJ": "J", "ballast": "kg", "maxTorque": "Nm",
    "tc": "raw", "abs": "raw", "tyreWear": "raw", "carDamage": "raw",
}
FLAGS = {"pitLimiterOn", "autoShifterOn", "ersHeatCharging", "ersIsCharging", "drsAvailable", "drsEnabled", "isAIControlled", "isInPit", "idealLineOn", "isInPitLine", "mandatoryPitDone", "penaltiesEnabled", "aidAllowTyreBlankets", "aidAutoClutch", "aidAutoBlip", "hasDRS", "hasERS", "hasKERS", "isTimedRace", "hasExtraLap"}


def generate(source):
    layouts = []
    classes = {"SPageFilePhysics": (0,"physics","AC Physics"), "SPageFileGraphic": (1,"graphics","AC Graphics"), "SPageFileStatic": (2,"static","AC Static")}
    for cls in ast.parse(source).body:
        if not isinstance(cls, ast.ClassDef) or cls.name not in classes:
            continue
        packet_id, page, group = classes[cls.name]
        declaration = next(n.value for n in cls.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=="_fields_" for t in n.targets))
        offset, fields = 0, []
        for entry in declaration.elts:
            name = ast.literal_eval(entry.elts[0])
            kind, dims = shape(entry.elts[1])
            if name.startswith("tyreContact"):
                dims = [4,3] # C++ wheel[4][XYZ=3]; Python helper's multiplication reverses the shape.
            element_size = 2 if kind=="c_wchar" else 4
            align = min(element_size,4)
            offset = (offset+align-1)//align*align
            length = element_size
            for n in dims: length *= n
            dimensions = []
            if kind!="c_wchar":
                stride = length
                for i,n in enumerate(dims):
                    stride //= n
                    if name.startswith("tyreContact"):
                        dim_label = "Wheel (FL / FR / RL / RR)" if i==0 else "Axis (X / Y / Z)"
                    elif n==4: dim_label = "Wheel (FL / FR / RL / RR)"
                    elif n==3: dim_label = "Axis (X / Y / Z)"
                    elif name=="carDamage": dim_label = "Zone (front / rear / left / right / centre)"
                    else: dim_label = "Item"
                    dimensions.append({"count":n,"stride":stride,"label":dim_label})
            if name!="packetId":
                output_kind = "utf16" if kind=="c_wchar" else "float" if kind=="c_float" else "int32"
                unit = UNITS.get(name,"")
                states = {"0":"OFF","1":"ON"} if name in FLAGS else {}
                if name=="status": states={"0":"OFF","1":"REPLAY","2":"LIVE","3":"PAUSED"}
                if name=="session": states={"-1":"UNKNOWN","0":"PRACTICE","1":"QUALIFY","2":"RACE","3":"HOTLAP","4":"TIME ATTACK","5":"DRIFT","6":"DRAG"}
                if name=="flag": states={"0":"NONE","1":"BLUE","2":"YELLOW","3":"BLACK","4":"WHITE","5":"CHECKERED","6":"PENALTY"}
                f={"key":f"ac.{page}.{name}","label":group+" · "+label(name),"offset":offset,"kind":output_kind,"dimensions":dimensions,"unit":unit,"description":f"Assetto Corsa {page}: {name}","states":states}
                if unit=="%":
                    f["scale"]=100.
                    f["description"]+=" (0–1 converted to %)"
                if name=="tyreWear": f["description"]+=" (raw game value; not converted to percentage worn)"
                if name in {"tc","abs"}: f["description"]+=" (raw signal, not the F1 assist level)"
                if kind=="c_wchar": f["text_len"]=length//2
                fields.append(f)
            offset+=length
        size=(offset+3)//4*4
        expected={0:580,1:296,2:684}[packet_id]
        if size!=expected: raise ValueError(f"{page}: expected {expected}, calculated {size}")
        layouts.append({"id":packet_id,"size":size,"group":group,"fields":fields})
    if len(layouts)!=3: raise ValueError("Three AC structures are required")
    return layouts


if __name__=="__main__":
    layouts=generate(Path(sys.argv[1]).read_text(encoding="utf-8-sig"))
    Path(sys.argv[2]).write_text(json.dumps(layouts,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
    for p in layouts: print(f'{p["group"]}: {p["size"]} bytes, {len(p["fields"])} fields')
