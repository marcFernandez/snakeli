# Snakeli

> First part streamed on https://twitch.tv/zartisimo ([video](https://www.twitch.tv/videos/2025056082))

I was bored. Feel free to copy the crappy code, I don't care :D

![](docs/demo_v2.png)

## Usage

```
Snakeli - v1

snakeli [-w 50] [-h 30] [-l 5] [-m TRIM]

    --help  print this help
        -w  width of the board
        -h  height of the board
        -l  initial length. It has to be less than w-2 (48 by default)
        -m  game mode. REGULAR by default:
              - TRIM: Snake eats itself
              - REGULAR: Snake eats itself
```

## Features

1. Configurable board size
2. Pausable
3. 2 game modes
    3.1. REGULAR: If snake hits itself, you lose
    3.2. TRIM: If snake hits itself, it loses from the hit to its tail
4. Restart
5. Score

## References

- Game loop: https://gameprogrammingpatterns.com/game-loop.html

## TODOs

1. Add multiplayer (friendly or unfriendly?)
2. Do not re-render the entire screen every single time and render updates only (TBD)
3. ¯\\_(ツ)_/¯

