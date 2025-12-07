#!/usr/bin/env node

import { main } from "@/cli/cli.js";
import { exitWith } from "@/utils/debug.js";

main().catch(exitWith);
