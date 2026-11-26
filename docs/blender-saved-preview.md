# Blender saved-image preview
The first pass displays only the image saved in the initial metadata of an
uncompressed, little-endian Blender file. It never opens Blender, evaluates
a scene, calls the registered Windows thumbnail provider, loads addons,
downloads links or executes anything stored in the file. It needs no new
service, settings path or application dependency.

Hover reads up to 2 MiB of an ordinary local source and transfers only those
bytes to the existing contained decoder. The saved image is limited to 512
pixels per side. Other layouts, compressed files and absent or unreadable
saved images receive a labeled metadata fallback. This is not interactive
3D and does not include dependency, material or swatch extraction.

The public format references are Blender's [header](https://github.com/blender/blender/blob/v5.3.0/source/blender/blenloader_core/BLO_core_blend_header.hh)
and [block layout](https://github.com/blender/blender/blob/v5.3.0/source/blender/blenloader_core/BLO_core_bhead.hh).
The reader is original Hover code. Synthetic fixtures cover each raw layout,
asymmetric orientation and channel mapping, early metadata stop, and the
actual native painter and worker boundary. Real creator-file/oracle
comparison and compressed compatibility are separate qualification work.
