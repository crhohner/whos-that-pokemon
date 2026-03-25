use color_eyre::{
    Result,
    eyre::{bail, eyre},
    owo_colors::colors::Magenta,
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
    // generation: PokemonGeneration,
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
    //todo - grab the default information instead
    weight: i32,
    types: Vec<PokeTypeSlot>,
    sprites: PokeSpriteURL,
    // generation: String,
    // shape: String,
}

#[derive(Deserialize, Debug, Default)]
struct PokeSpriteURL {
    front_default: String,
}

#[derive(Deserialize, Debug)]
struct PokeTypeSlot {
    r#type: PokeType,
}

#[derive(Deserialize, Debug)]
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

    pub fn get_info(&mut self, poke_index: usize) -> Result<Pokemon> {
        let pokemon_url = &self.results[poke_index].url;

        let species_json = reqwest::blocking::get(pokemon_url)?
            .error_for_status()?
            .json::<PokemonSpecies>()?;

        let species = species_json
            .varieties
            .iter()
            .find(|var| var.is_default)
            .ok_or_else(|| {
                eyre!(
                    "no default pokemon found for species {}",
                    &self.results[poke_index].name
                )
            })?;

        let pokemon = reqwest::blocking::get(&species.pokemon.url)?
            .error_for_status()?
            .json::<Pokemon>()?;
        // pokemon.generation = species_json.generation.name;
        // pokemon.shape = species_json.shape.name;
        Ok(pokemon)
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
    pokemon: Pokemon,
    input_mode: InputMode,
    input: Input,
    button_focus: usize,
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
                    KeyCode::Esc => self.exit = true,
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
                            1 => {} //pass for now
                            // skip
                            2 => self.next_pokemon()?,
                            _ => bail!("invalid button press!!"),
                        }
                    }
                    _ => {}
                },
                InputMode::Guess => match key.code {
                    KeyCode::Enter => self.check_guess(),
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

    fn check_guess(&mut self) {
        debug!("{}", self.input.value_and_reset());
    }

    fn next_pokemon(&mut self) -> color_eyre::Result<()> {
        let n = self.rng.random_range(0..self.pokedex.count as usize);
        self.pokemon = self.pokedex.get_info(n)?;
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
                Constraint::Percentage(70),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
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

        let [_, b1, b2, b3, _] = Layout::default()
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
                button_style = button_style.add_modifier(Modifier::REVERSED);
            }
            Paragraph::new(button_text[i])
                .style(button_style)
                .block(Block::bordered())
                .alignment(Alignment::Center)
                .render(button_areas[i], buf);
        }
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
- make all ui arrow keys + enter + escape (buttons)
- five guess worldle-like "game" + hints
- win/lose results page (just the prev round for now)
- ascii converter / crush at diff resolutions
- hints / increase resolution at each guess
- cleanup
*/
