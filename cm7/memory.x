MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 1024K
  RAM   : ORIGIN = 0x24000000, LENGTH = 512K
}

SECTIONS
{
  .uninit (NOLOAD) :
  {
    . = ALIGN(8);
    __suninit = .;

    *(.uninit .uninit.*);

    . = ALIGN(8);
    __euninit = .;
  } > RAM
}