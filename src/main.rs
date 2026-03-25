use std::fmt::Debug;

use color_eyre::{
    Result,
    eyre::{bail, eyre},
};
use crossterm::event::{self, Event, KeyCode};
use log::debug;
use rand::{RngExt, rngs::ThreadRng};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    symbols::border,
    text::Line,
    widgets::{Block, Paragraph, Widget},
};
use ratatui::{
    layout::Alignment,
    style::{Color, Style},
};
use ratatui::{
    layout::{Margin, Position},
    text::Text,
};
use serde::{Deserialize, Serialize};
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;
use tui_logger::*;

const API: &str = "https://pokeapi.co/api/v2/";

const ASCII_TEST: &str = "                                                                                                    
                                                                                                    
                                                                                                                                   
                                                                                                    
                                           ++=+##@  #@@@@                                           
                                    ++*##%#*-:-=++#*::-=#@@                                         
                                   +-::-=======++===--===%@                                         
                                  #+-:-=================++*#%                                       
                                  @%+======+*======#========+**%                                    
                                   @#+=====+++=================*@                                   
                                    @#========*%@@@%*==========*@                                   
                                    %*========================*%%                                   
                                   %*=======================+++*#                                   
                                 @@*=========================+++#@                                  
                                #*+==========================++++*#                                 
                               %**+++=======================++++++*#@                               
                               @#+++++++++++==========+++++++++++++*@                               
                                @#++++++++++++====+++=+=+++++++++++#@                               
                                 @@#++#@@##++++++++++++++#@@%*++#@@                                 
                                   @@@@  %##%+++++++++%%%%  %@@@%                                   
                                            @@@@@@@@@@@                                             
                                                                                                    
                                                                                                    
                                                                                                    ";

#[derive(Serialize, Deserialize, Debug, Default)]
struct PokedexEntry {
    name: String,
    url: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Pokedex {
    count: i32, //remove if conclusively cant be used to generate indices
    results: Vec<PokedexEntry>,
}

#[derive(Deserialize, Debug, Default)]
struct PokemonSpecies {
    varieties: Vec<PokemonVariety>,
    generation: PokemonGeneration,
    shape: PokemonShape,
}

#[derive(Deserialize, Debug, Default)]
struct PokemonGeneration {
    name: String,
}

#[derive(Deserialize, Debug, Default)]
struct PokemonShape {
    name: String,
}

#[derive(Deserialize, Debug, Default)]
struct PokemonVariety {
    is_default: bool,
    pokemon: PokedexEntry,
}

#[derive(Deserialize, Debug, Default)]
struct Pokemon {
    name: String,
    weight: i32,
    types: Vec<PokeTypeSlot>,
    sprites: PokeSpriteURL,
}

#[derive(Deserialize, Serialize, Debug, Default)]
struct PokeData {
    name: String,
    //todo - grab the default information instead
    weight: i32,
    types: Vec<String>,
    sprite: String,
    generation: String,
    shape: String,
}

impl PokeData {
    fn new(pokemon: Pokemon, species: PokemonSpecies) -> Self {
        Self {
            name: pokemon.name.replace("-", " "),
            weight: pokemon.weight,
            types: pokemon
                .types
                .iter()
                .map(|typ| typ.r#type.name.clone())
                .collect::<Vec<_>>(),
            sprite: pokemon.sprites.front_default,
            generation: species.generation.name.replace("-", " "),
            shape: species.shape.name,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Default)]
struct PokeSpriteURL {
    front_default: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct PokeTypeSlot {
    r#type: PokeType,
}

#[derive(Deserialize, Serialize, Debug)]
struct PokeType {
    name: String,
}

impl Pokedex {
    pub fn init(&mut self) -> Result<()> {
        //ignore this slightly strange pattern pls

        let pokedex_url = format!("{}{}", API, "pokemon-species?limit=100000&offset=0");

        let json = reqwest::blocking::get(pokedex_url)?
            .error_for_status()?
            .json::<Pokedex>()?;

        *self = json;
        Ok(())
    }

    pub fn get_info(&mut self, poke_index: usize) -> Result<PokeData> {
        let pokemon_url = &self.results[poke_index].url;

        let species = reqwest::blocking::get(pokemon_url)?
            .error_for_status()?
            .json::<PokemonSpecies>()?;

        let default = species
            .varieties
            .iter()
            .find(|var| var.is_default)
            .ok_or_else(|| {
                eyre!(
                    "no default pokemon found for species {}",
                    &self.results[poke_index].name
                )
            })?;

        let pokemon = reqwest::blocking::get(&default.pokemon.url)?
            .error_for_status()?
            .json::<Pokemon>()?;

        let pokedata = PokeData::new(pokemon, species);
        Ok(pokedata)
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
enum InputMode {
    #[default]
    Navigate,
    Guess,
}

#[derive(Debug, Default)]
pub struct App {
    pokedex: Pokedex,
    exit: bool,
    rng: ThreadRng,
    pokemon: PokeData,
    input_mode: InputMode,
    input: Input,
    button_focus: usize,
    hints_used: usize,
}

/*
- image
- weight type generation
- aka **** **** *****
- then [hint] [skip] [quit]
- then the text input

*/

impl App {
    pub fn init(&mut self) -> color_eyre::Result<()> {
        self.pokedex.init()?;
        self.rng = rand::rng();
        self.next_pokemon()?;
        self.input_mode = InputMode::Navigate;
        Ok(())
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        self.init()?;

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
        if self.input_mode == InputMode::Guess {
            let (_, _, _, _, input_area, _) = &self.layout(frame.area());
            frame.set_cursor_position(Position::new(
                self.input.visual_cursor() as u16 + input_area.x + 1,
                input_area.y + 1,
            ));
        }
    }

    fn handle_events(&mut self) -> color_eyre::Result<()> {
        let event = event::read()?;

        if let Event::Key(key) = event {
            match self.input_mode {
                InputMode::Navigate => match key.code {
                    KeyCode::Up => self.focus_guess(),
                    KeyCode::Right => {
                        self.button_focus = (self.button_focus + 1) % 3;
                    }
                    KeyCode::Left => {
                        self.button_focus = (self.button_focus + 2) % 3;
                    }
                    KeyCode::Enter => {
                        match self.button_focus {
                            // quit
                            0 => self.exit = true,
                            // hint
                            1 => {
                                self.hints_used += 1;
                                if self.hints_used > 4 {
                                    self.next_pokemon()?
                                }
                            } //pass for now
                            // skip
                            2 => self.next_pokemon()?,
                            _ => bail!("invalid button press!!"),
                        }
                    }
                    _ => {}
                },
                InputMode::Guess => match key.code {
                    KeyCode::Enter => self.check_guess()?,
                    KeyCode::Down => self.focus_nav(),
                    _ => {
                        self.input.handle_event(&event);
                    }
                },
            }
        }

        Ok(())
    }

    fn focus_guess(&mut self) {
        self.input_mode = InputMode::Guess
    }

    fn focus_nav(&mut self) {
        self.input_mode = InputMode::Navigate;
        self.button_focus = 0;
    }

    fn check_guess(&mut self) -> Result<()> {
        let guess = self.input.value_and_reset();
        if guess.to_lowercase() == self.pokemon.name {
            self.next_pokemon()?
        }
        Ok(())
    }

    fn next_pokemon(&mut self) -> color_eyre::Result<()> {
        let n = self.rng.random_range(0..self.pokedex.count as usize);
        self.pokemon = self.pokedex.get_info(n)?;
        debug!(
            "{:#?}",
            serde_json::to_string_pretty(&self.pokemon).unwrap()
        );
        self.hints_used = 0;
        Ok(())
    }

    fn layout(&self, area: Rect) -> (Rect, Rect, Rect, Rect, Rect, Rect) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);
        let logs_area = main_layout[0];
        let game_area = main_layout[1];

        let [canvas_area, hints_area, input_area, buttons_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .areas(game_area.inner(ratatui::layout::Margin {
                horizontal: (2),
                vertical: (1),
            }));

        return (
            logs_area,
            game_area,
            canvas_area,
            hints_area,
            input_area,
            buttons_area,
        );
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (logs_area, game_area, canvas_area, hints_area, input_area, buttons_area) =
            self.layout(area);

        //render logs
        TuiLoggerWidget::default()
            .block(Block::bordered().title("logs"))
            .output_separator('|')
            .output_level(Some(TuiLoggerLevelOutput::Long))
            .style(Style::default().fg(Color::White))
            .render(logs_area, buf);

        //render border

        let title = Line::from("WHO'S THAT POKEMON?")
            .style(Style::default().add_modifier(Modifier::REVERSED));

        let block = Block::bordered()
            .title(title.centered())
            .border_set(border::THICK);

        block.render(game_area, buf);

        //render ditto

        Paragraph::new(ASCII_TEST)
            .centered()
            .render(canvas_area, buf);

        //render input

        let input_style = match self.input_mode {
            InputMode::Navigate => Style::default(),
            InputMode::Guess => Color::LightCyan.into(),
        };

        Paragraph::new(self.input.value())
            .style(input_style)
            .block(Block::bordered().title("guess!!"))
            .render(input_area, buf);

        //render buttons

        let [left, b1, b2, b3, right] = Layout::default() //scuffed
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Min(0),
            ])
            .areas(buttons_area);

        let button_areas = [b1, b2, b3];

        let button_text = ["quit", "hint", "skip"];

        for i in 0..3 {
            let mut button_style = Style::default();
            if self.input_mode == InputMode::Navigate && self.button_focus == i {
                button_style = button_style.light_cyan();
            }
            Paragraph::new(button_text[i])
                .style(button_style)
                .block(Block::bordered())
                .alignment(Alignment::Center)
                .render(button_areas[i], buf);
        }

        Paragraph::new(". ݁₊ ⊹ . ݁ ⟡ ݁ . ⊹ ₊ ݁.⋆˙⟡ ⋆.˚ ⊹₊⟡ ⋆")
            .block(Block::bordered())
            .alignment(Alignment::Center)
            .render(left, buf);

        Paragraph::new(". ݁₊ ⊹ . ݁ ⟡ ݁ . ⊹ ₊ ݁.⋆˙⟡ ⋆.˚ ⊹₊⟡ ⋆")
            .block(Block::bordered())
            .alignment(Alignment::Center)
            .render(right, buf);

        let mut hints = vec![
            self.pokemon.generation.clone(),
            self.pokemon.shape.clone(),
            self.pokemon.weight.to_string(),
            format!("{:?}", self.pokemon.types),
        ];

        for i in self.hints_used..4 {
            hints[i] = "*****".to_string();
        }

        //render hints
        Paragraph::new(format!(
            "{} | {} | {} | types: {}",
            hints[0], hints[1], hints[2], hints[3],
        ))
        .alignment(Alignment::Center)
        .render(hints_area, buf);
    }
}

fn main() -> color_eyre::Result<()> {
    tui_logger::init_logger(LevelFilter::Debug).unwrap();
    tui_logger::set_default_level(LevelFilter::Debug);

    color_eyre::install()?;

    ratatui::run(|terminal| App::default().run(terminal))
}

//TODO
/*
- fix input extra keystroke bug (table this i guess? esp if its a logging issue..)

- fix gen/shape issue
- hints (as speech bubbles?)
- win ack and/or history
- ascii converter / crush at diff resolutions
- display image(s)
- cleanup
*/
