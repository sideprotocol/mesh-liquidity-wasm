# LSD contract for SIDE token

## Overview

LSD will provide the layer of smart contracts between delegator and validator which auto-compounds the rewards after a fixed interval without requirement of delegator intervention and provide a derivative sdSIDE which can be used in DeFi.
This uses accuring rewards model i.e all the rewards are accumulated in sdSIDE itself, and sdSIDE value increases over time. Increase in value depends on staking reward rate of chain.

## Features

- Ability to convert staked SIDE into sdSIDE is done by issuing a staking derivative token which represents the user’s delegator’s stake and rewards accumulated.
- This staking derivative token can be swapped with sdSIDE using a DEX supporting such pairs or used in lending.

