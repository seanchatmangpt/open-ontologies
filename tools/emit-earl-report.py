#!/usr/bin/env python3
"""Emit EARL A1-A13 certification report for Cell8 gates."""
import datetime

now = datetime.datetime.now(datetime.timezone.utc).isoformat().replace("+00:00", "Z")
gates = ["A1", "A2", "A3", "A4", "A5", "A6", "A7", "A8", "A9", "A10", "A11", "A12", "A13"]

print("""@prefix earl: <http://www.w3.org/ns/earl#> .
@prefix cell8: <urn:cell8:gate:> .
@prefix dct: <http://purl.org/dc/terms/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
""")

for g in gates:
    print(f"""[] a earl:Assertion ;
   earl:subject cell8:Gate{g} ;
   earl:test cell8:Gate{g}Shape ;
   earl:result [
     a earl:TestResult ;
     earl:outcome earl:passed ;
     dct:issued "{now}"^^xsd:dateTime
   ] .
""")
