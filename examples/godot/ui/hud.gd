extends Control

# Almacén del muelle: panel con título y cuerpo, y una etiqueta de HUD sobre el mundo.
func _ready() -> void:
	var panel := PanelContainer.new()
	panel.theme_type_variation = &"HarborPanel"
	add_child(panel)

	var box := VBoxContainer.new()
	panel.add_child(box)

	var title := Label.new()
	title.theme_type_variation = &"HarborTitle"
	title.text = "Almacén del muelle"
	box.add_child(title)

	var body := Label.new()
	body.theme_type_variation = &"HarborBody"
	body.text = "Hoy llegan 40 cajas de bacalao. Hay que salarlas antes de la noche."
	box.add_child(body)

	var clock := Label.new()
	clock.theme_type_variation = &"HarborHud"
	clock.text = "Marea baja a las 18:30"
	add_child(clock)
