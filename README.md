# FIUBA - Taller de Programacion I
## Proyecto de Nodo Bitcoin y Wallet

### Descripción
El objetivo principal de este proyecto de desarrollo es la implementación de un Nodo Bitcoin con funcionalidades acotadas. Además, se desarrollará una interfaz de una wallet que permitirá crear transacciones, enviarlas y monitorearlas.

### Documentación
#### Diagrama General

El ciclo de vida del nodo se describe a continuación:

Nodo::new(config): Recibe la configuración y crea el nodo junto con sus estructuras principales.
node.initialize(): Realiza el handshake con todos los nodos y actualiza la blockchain.
node.listen(): Consume el nodo, crea un hilo para cada stream y maneja los mensajes de cada uno.
Es importante destacar que cada campo del nodo sensible al multithreading está encapsulado por su respectivo RwLock o Mutex, como la blockchain.

### Nodo

Handshake
El Handshake se implementó para habilitar la capacidad de enviar y recibir mensajes version y verack, fundamentales para la comunicación entre nodos en la red de Bitcoin. Se diseñaron estructuras específicas para cada uno de estos mensajes, siguiendo las especificaciones de la guía de desarrollo de Bitcoin.

Mensajes
Todas las estructuras de mensajes utilizadas en el sistema comparten los mismos métodos fundamentales, como to_bytes(), write_to(stream) y read_from(stream). Estos métodos garantizan un formato común para la representación y manipulación de los datos.

Blockchain
La estructura blockchain se encarga de guardar todos los bloques y transacciones en memoria, así como de mantener el set de UTXO (Outputs sin gastar). Para representarla en memoria, se utiliza una lista enlazada y se maneja la cadena de headers en un archivo para futuras ejecuciones del nodo.

Obtención de Headers
Se utiliza el método "headers-first" para la obtención de headers, conectándose con un nodo de sincronización inicial y recibiendo los encabezados para su procesamiento y almacenamiento.

Obtención de Bloques
Se implementa la descarga de bloques entre varios hilos para una mayor eficiencia y paralelización del proceso.

### Wallet
La interfaz gráfica de la wallet se implementa utilizando GTK y Glade. Se establece una comunicación entre la wallet y el nodo para realizar diversas operaciones, como obtener el balance de una cuenta, el historial de transacciones y realizar pagos.

### Cuentas
Se modelan las cuentas de los usuarios con atributos como la dirección, la clave privada (WIF), el saldo, el saldo pendiente y las transacciones asociadas.
