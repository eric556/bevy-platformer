# Notes

## Movement
- Air movment speed should be directly proportional to the floatiness of gravity (less gravity, more air manuverability)
- Add a breaking force when the user lets go of movment buttons IN ADDITION to friction of the surface they are on. When people want to stop moving the add forces to the friction to stop

## Collisions
- Seperate into 3 phase, broad and narrow, and resolution
    - broad phase: determine objects that may end up hitting using some sort of spatial data structure (QTree)
    - narrow phase: determine if the objects that were deemed to possible be hitting overlap and calculate the projection vector
    - resolution phase: resolve the collisions by modifying position (projection), velocity (immediate impulse), acceleration (penalty)
- Going with projection based, you need to know how deep the object penetrated into the other
- Tunneling is an issue, but this can be solved by running the physics at a faster timestep


## Todo
- Broad phase of collision detection (Big aabb area that surrounds the start and destenation of the moving object)
- Move all physics into one system. Fuck this noise
- Basic analoug jumping
- Wall jumping
- Change animations based on player input (state machine for char)
- Level tilemap loading
- Collisions with level
- Backgrounds
- Parallax on backgrounds