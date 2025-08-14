#!/usr/bin/env bash
avocado sdk run -ie --container-arg "--net=host" vm dev --mem 512
