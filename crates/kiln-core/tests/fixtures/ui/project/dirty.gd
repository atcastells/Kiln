extends Control
# Color(1, 0, 0) in a comment is not a style written outside the theme

func _ready():
	$A.theme_type_variation = &"Faint"
	$B.theme_type_variation = &"Plain"
	$C.theme_type_variation = &"Ghost"
	$A.text = "warning \"⚠\" here"  # trailing comment, text stays
	$A.modulate = Color(1, 0, 0)
