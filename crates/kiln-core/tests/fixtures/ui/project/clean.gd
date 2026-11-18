extends Control

func _ready():
	$A.theme_type_variation = &"Good"  # Color(1, 0, 0) here is only a comment
	$A.text = "it's all 'fine' # not a comment"
