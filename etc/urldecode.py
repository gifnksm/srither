#!/usr/bin/env python2

import sys, urllib

print urllib.unquote(sys.stdin.read())
