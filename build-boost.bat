@echo off
cls
cd external/boost
b2 -j 4 --build-dir=../boost-lib64 --stagedir=./lib64 toolset=msvc-12.0 link=static threading=multi runtime-link=shared --build-type=minimal --with-date_time --with-math --with-python --with-system --with-thread address-model=64 stage
rd /q /s ..\boost-lib64
pause